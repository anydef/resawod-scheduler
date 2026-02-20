use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use chrono::{NaiveDateTime, NaiveTime};
use tracing::{error, info, warn};

use super::views::capitalize;
use super::{SchedulerEntry, SchedulerState};
use crate::client::NubappClient;
use crate::models::{Config, User};
use crate::scheduler;

enum BookingOutcome {
    Booked,
    AlreadyBooked,
    WaitingList,
    SlotNotFound,
    Failed(String),
}

fn load_booked_slots(path: &Path) -> HashSet<String> {
    match std::fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => HashSet::new(),
    }
}

fn save_booked_slots(path: &Path, slots: &HashSet<String>) {
    if let Ok(json) = serde_json::to_string_pretty(slots) {
        if let Err(e) = std::fs::write(path, json) {
            error!(
                "Failed to save scheduler state to {}: {}",
                path.display(),
                e
            );
        }
    }
}

pub(crate) fn spawn_slot_schedulers(
    config: Arc<Config>,
    entries: SchedulerState,
    state_path: PathBuf,
) {
    let existing = load_booked_slots(&state_path);
    info!(
        "Scheduler: loaded {} booked slots from {}",
        existing.len(),
        state_path.display()
    );
    let booked: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(existing));
    let state_path = Arc::new(state_path);

    for user in &config.users {
        for day_name in &user.slots {
            let slot_cfg = match config.slots.get(day_name) {
                Some(c) => c.clone(),
                None => {
                    warn!(
                        "Scheduler: no slot configured for '{}', skipping",
                        day_name
                    );
                    continue;
                }
            };
            if scheduler::parse_weekday(day_name).is_none() {
                warn!("Scheduler: unknown day '{}', skipping", day_name);
                continue;
            }

            info!(
                "Scheduler: spawning task for {} — {} {} ({})",
                user.name,
                day_name,
                slot_cfg.time,
                slot_cfg.activity.as_deref().unwrap_or("any")
            );
            tokio::spawn(slot_booking_task(
                Arc::clone(&config),
                user.clone(),
                day_name.clone(),
                slot_cfg.time.clone(),
                slot_cfg.activity.clone(),
                Arc::clone(&entries),
                Arc::clone(&booked),
                Arc::clone(&state_path),
            ));
        }
    }
}

fn update_scheduler_entry(entries: &SchedulerState, key: &str, entry: SchedulerEntry) {
    entries.lock().unwrap().insert(key.to_string(), entry);
}

async fn attempt_slot_booking(
    config: &Config,
    user: &User,
    slot_time_str: &str,
    activity: Option<&str>,
    target_date: chrono::NaiveDate,
) -> Result<BookingOutcome> {
    let mut nubapp =
        NubappClient::new(&config.app.application_id, &config.app.category_activity_id)?;
    nubapp.login(&user.login, &user.password).await?;

    // Check existing bookings to avoid double-booking
    let bookings_resp = nubapp.get_bookings().await?;
    let data = bookings_resp.get("data");
    let target_ymd = target_date.format("%Y-%m-%d").to_string();
    let activity_filter = activity.filter(|a| !a.is_empty());

    if let Some(arr) = data
        .and_then(|d| d.get("bookings"))
        .and_then(|v| v.as_array())
    {
        for b in arr {
            let start = b
                .get("start_timestamp")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if start.contains(&target_ymd) && start.contains(slot_time_str) {
                if let Some(af) = activity_filter {
                    let name = b
                        .get("name_activity")
                        .or_else(|| b.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if name.to_lowercase().contains(&af.to_lowercase()) {
                        return Ok(BookingOutcome::AlreadyBooked);
                    }
                } else {
                    return Ok(BookingOutcome::AlreadyBooked);
                }
            }
        }
    }

    // Fetch available slots for the target date
    let api_date = target_date.format("%d-%m-%Y").to_string();
    let slots = nubapp.get_slots(&api_date).await?;

    let slot = match NubappClient::find_slot(&slots, slot_time_str, activity) {
        Some(s) => s,
        None => return Ok(BookingOutcome::SlotNotFound),
    };

    let slot_id = slot
        .id_activity_calendar
        .to_string()
        .trim_matches('"')
        .to_string();

    // Try direct booking
    let resp = nubapp.book(&slot_id).await?;
    let success = resp
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if success {
        return Ok(BookingOutcome::Booked);
    }

    let msg = resp
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Slot full — try waiting list
    info!(
        "Scheduler: direct book failed for {} ({}), trying waiting list",
        user.name, msg
    );
    let wl_resp = nubapp.book_waiting_list(&slot_id).await?;
    let wl_ok = wl_resp
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if wl_ok {
        return Ok(BookingOutcome::WaitingList);
    }

    Ok(BookingOutcome::Failed(msg))
}

async fn slot_booking_task(
    config: Arc<Config>,
    user: User,
    day_name: String,
    slot_time_str: String,
    activity: Option<String>,
    entries: SchedulerState,
    booked: Arc<Mutex<HashSet<String>>>,
    state_path: Arc<PathBuf>,
) {
    let weekday = scheduler::parse_weekday(&day_name).unwrap();
    let time_trimmed = slot_time_str.trim();
    let slot_time = NaiveTime::parse_from_str(time_trimmed, "%H:%M:%S")
        .or_else(|_| NaiveTime::parse_from_str(time_trimmed, "%H:%M"))
        .unwrap_or_else(|e| {
            panic!("Cannot parse slot time '{}': {}", slot_time_str, e);
        });
    let booking_time = slot_time + chrono::Duration::minutes(1);
    let entry_key = format!("{}:{}", user.name, day_name);

    loop {
        let now = scheduler::now();
        let today = now.date_naive();
        let target_date = scheduler::next_weekday(today, weekday);
        let slot_key = format!("{}:{}:{}", user.login, target_date, slot_time_str);

        // Booking window: 7 days before target at slot_time + 1 min
        let opens_date = target_date - chrono::Duration::days(7);
        let opens_naive = NaiveDateTime::new(opens_date, booking_time);
        let opens_at = opens_naive
            .and_local_timezone(scheduler::CET)
            .earliest()
            .unwrap();

        let target_str = target_date.format("%Y-%m-%d").to_string();
        let opens_str = opens_at.format("%Y-%m-%d %H:%M").to_string();

        // Already booked for this target — advance to next window
        if booked.lock().unwrap().contains(&slot_key) {
            let next_window = NaiveDateTime::new(target_date, booking_time)
                .and_local_timezone(scheduler::CET)
                .earliest()
                .unwrap();
            update_scheduler_entry(
                &entries,
                &entry_key,
                SchedulerEntry {
                    user_name: user.name.clone(),
                    day: capitalize(&day_name),
                    time: slot_time_str.clone(),
                    target_date: target_str,
                    books_at: opens_str,
                    status: "booked".into(),
                },
            );
            if next_window > scheduler::now() {
                let dur = (next_window - scheduler::now())
                    .to_std()
                    .unwrap_or(Duration::from_secs(60));
                tokio::time::sleep(dur).await;
            } else {
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
            continue;
        }

        // Update dashboard: scheduled
        update_scheduler_entry(
            &entries,
            &entry_key,
            SchedulerEntry {
                user_name: user.name.clone(),
                day: capitalize(&day_name),
                time: slot_time_str.clone(),
                target_date: target_str.clone(),
                books_at: opens_str.clone(),
                status: "scheduled".into(),
            },
        );

        // Sleep until booking window opens
        if opens_at > now {
            info!(
                "Scheduler: {} {} for {} — booking at {} for {}",
                day_name, slot_time_str, user.name, opens_str, target_str
            );
            let dur = (opens_at - now)
                .to_std()
                .unwrap_or(Duration::from_secs(60));
            tokio::time::sleep(dur).await;
        }

        // Attempt booking
        update_scheduler_entry(
            &entries,
            &entry_key,
            SchedulerEntry {
                user_name: user.name.clone(),
                day: capitalize(&day_name),
                time: slot_time_str.clone(),
                target_date: target_str.clone(),
                books_at: opens_str.clone(),
                status: "booking...".into(),
            },
        );

        match attempt_slot_booking(
            &config,
            &user,
            &slot_time_str,
            activity.as_deref(),
            target_date,
        )
        .await
        {
            Ok(BookingOutcome::Booked) => {
                info!(
                    "Scheduler: booked {} {} for {} on {}",
                    day_name, slot_time_str, user.name, target_str
                );
                let mut set = booked.lock().unwrap();
                set.insert(slot_key);
                save_booked_slots(&state_path, &set);
                drop(set);
                update_scheduler_entry(
                    &entries,
                    &entry_key,
                    SchedulerEntry {
                        user_name: user.name.clone(),
                        day: capitalize(&day_name),
                        time: slot_time_str.clone(),
                        target_date: target_str,
                        books_at: opens_str,
                        status: "booked".into(),
                    },
                );
            }
            Ok(BookingOutcome::AlreadyBooked) => {
                info!(
                    "Scheduler: {} already booked {} {} on {}",
                    user.name, day_name, slot_time_str, target_str
                );
                booked.lock().unwrap().insert(slot_key);
                update_scheduler_entry(
                    &entries,
                    &entry_key,
                    SchedulerEntry {
                        user_name: user.name.clone(),
                        day: capitalize(&day_name),
                        time: slot_time_str.clone(),
                        target_date: target_str,
                        books_at: opens_str,
                        status: "already booked".into(),
                    },
                );
            }
            Ok(BookingOutcome::WaitingList) => {
                info!(
                    "Scheduler: {} added to waiting list for {} {} on {}",
                    user.name, day_name, slot_time_str, target_str
                );
                booked.lock().unwrap().insert(slot_key);
                update_scheduler_entry(
                    &entries,
                    &entry_key,
                    SchedulerEntry {
                        user_name: user.name.clone(),
                        day: capitalize(&day_name),
                        time: slot_time_str.clone(),
                        target_date: target_str,
                        books_at: opens_str,
                        status: "full, joined waiting list".into(),
                    },
                );
            }
            Ok(BookingOutcome::SlotNotFound) => {
                warn!(
                    "Scheduler: slot not found {} {} for {} on {}",
                    day_name, slot_time_str, user.name, target_str
                );
                update_scheduler_entry(
                    &entries,
                    &entry_key,
                    SchedulerEntry {
                        user_name: user.name.clone(),
                        day: capitalize(&day_name),
                        time: slot_time_str.clone(),
                        target_date: target_str,
                        books_at: opens_str,
                        status: "slot not found".into(),
                    },
                );
                // Retry in 60s
                tokio::time::sleep(Duration::from_secs(60)).await;
                continue;
            }
            Ok(BookingOutcome::Failed(msg)) => {
                warn!(
                    "Scheduler: failed {} {} for {}: {}",
                    day_name, slot_time_str, user.name, msg
                );
                update_scheduler_entry(
                    &entries,
                    &entry_key,
                    SchedulerEntry {
                        user_name: user.name.clone(),
                        day: capitalize(&day_name),
                        time: slot_time_str.clone(),
                        target_date: target_str,
                        books_at: opens_str,
                        status: format!("failed: {msg}"),
                    },
                );
                tokio::time::sleep(Duration::from_secs(60)).await;
                continue;
            }
            Err(e) => {
                error!(
                    "Scheduler: error {} {} for {}: {:#}",
                    day_name, slot_time_str, user.name, e
                );
                update_scheduler_entry(
                    &entries,
                    &entry_key,
                    SchedulerEntry {
                        user_name: user.name.clone(),
                        day: capitalize(&day_name),
                        time: slot_time_str.clone(),
                        target_date: target_str,
                        books_at: opens_str,
                        status: format!("error: {e}"),
                    },
                );
                tokio::time::sleep(Duration::from_secs(60)).await;
                continue;
            }
        }

        // Successfully handled — sleep until next booking window opens
        let next_window = NaiveDateTime::new(target_date, booking_time)
            .and_local_timezone(scheduler::CET)
            .earliest()
            .unwrap();
        if next_window > scheduler::now() {
            let dur = (next_window - scheduler::now())
                .to_std()
                .unwrap_or(Duration::from_secs(60));
            tokio::time::sleep(dur).await;
        } else {
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
}
