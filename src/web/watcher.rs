use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, Local};
use tracing::{error, info, warn};

use crate::client::NubappClient;
use crate::models::{Config, User};

pub(crate) async fn waiting_list_watcher(
    config: Arc<Config>,
    last_check: Arc<Mutex<Option<DateTime<Local>>>>,
) {
    info!("Waiting-list watcher started (checking every 60 s)");
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        info!("Waiting-list watcher: running check");
        for user in &config.users {
            if let Err(e) = try_book_from_waiting_list(&config, user).await {
                error!("Watcher error for {}: {:#}", user.name, e);
            }
        }
        *last_check.lock().unwrap() = Some(Local::now());
    }
}

async fn try_book_from_waiting_list(config: &Config, user: &User) -> Result<()> {
    let mut nubapp =
        NubappClient::new(&config.app.application_id, &config.app.category_activity_id)?;
    nubapp.login(&user.login, &user.password).await?;

    let resp = nubapp.get_bookings().await?;
    let data = resp.get("data");

    let wl_entries: Vec<&serde_json::Value> = data
        .and_then(|d| d.get("in_waiting_list"))
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().collect())
        .unwrap_or_default();

    if wl_entries.is_empty() {
        return Ok(());
    }

    // Collect unique dates (YYYY-MM-DD) from waiting list timestamps
    let mut dates: Vec<String> = wl_entries
        .iter()
        .filter_map(|b| {
            b.get("start_timestamp")
                .and_then(|v| v.as_str())
                .and_then(|s| s.get(..10))
                .map(|s| s.to_string())
        })
        .collect();
    dates.sort();
    dates.dedup();

    // Fetch current capacity for all relevant slots
    let mut capacity_map: HashMap<String, (u32, u32)> = HashMap::new();
    for date in &dates {
        if let Some(api_date) = date
            .get(8..10)
            .zip(date.get(5..7))
            .zip(date.get(0..4))
            .map(|((d, m), y)| format!("{d}-{m}-{y}"))
        {
            if let Ok(slots) = nubapp.get_slots(&api_date).await {
                for slot in &slots {
                    let id = slot
                        .id_activity_calendar
                        .to_string()
                        .trim_matches('"')
                        .to_string();
                    if let (Some(ins), Some(cap)) = (slot.n_inscribed, slot.n_capacity) {
                        capacity_map.insert(id, (ins, cap));
                    }
                }
            }
        }
    }

    // For each waiting list entry, if there's a free spot, try to book it
    for entry in &wl_entries {
        let slot_id = match entry.get("id_activity_calendar") {
            Some(v) => v.to_string().trim_matches('"').to_string(),
            None => continue,
        };

        let start = entry
            .get("start_timestamp")
            .and_then(|v| v.as_str())
            .unwrap_or("?");

        if let Some(&(inscribed, capacity)) = capacity_map.get(&slot_id) {
            let free = capacity.saturating_sub(inscribed);
            if free > 0 {
                info!(
                    "Watcher: free spot for {} (slot {}, {} at {}/{}) — booking",
                    user.name, slot_id, start, inscribed, capacity
                );
                match nubapp.book(&slot_id).await {
                    Ok(resp) => {
                        let success = resp
                            .get("success")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        if success {
                            info!(
                                "Watcher: booked slot {} for {} (was on waiting list)",
                                slot_id, user.name
                            );
                        } else {
                            let msg = resp
                                .get("message")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            warn!(
                                "Watcher: booking slot {} for {} failed: {}",
                                slot_id, user.name, msg
                            );
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Watcher: booking request failed for {} slot {}: {:#}",
                            user.name, slot_id, e
                        );
                    }
                }
            }
        }
    }

    Ok(())
}
