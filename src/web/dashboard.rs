use std::collections::HashMap;

use axum::extract::State;
use axum::response::Html;

use super::views::render_page;
use super::{AppState, SchedulerEntry};
use crate::client::NubappClient;

pub(super) struct UserDashboard {
    pub(super) name: String,
    pub(super) bookings: Vec<BookingRow>,
    pub(super) waiting_list: Vec<WaitingRow>,
    pub(super) error: Option<String>,
}

pub(super) struct BookingRow {
    pub(super) start: String,
    pub(super) end: String,
    pub(super) name: String,
    pub(super) inscribed: Option<u32>,
    pub(super) capacity: Option<u32>,
}

pub(super) struct WaitingRow {
    pub(super) start: String,
    pub(super) end: String,
    pub(super) name: String,
    pub(super) inscribed: Option<u32>,
    pub(super) capacity: Option<u32>,
}

fn json_str(val: &serde_json::Value, keys: &[&str]) -> String {
    for key in keys {
        if let Some(s) = val.get(*key).and_then(|v| v.as_str()) {
            return s.trim().to_string();
        }
    }
    "?".to_string()
}

pub(crate) async fn dashboard_handler(State(state): State<AppState>) -> Html<String> {
    let cfg = &state.config;
    let mut users_data: Vec<UserDashboard> = Vec::new();

    for user in &cfg.users {
        let mut nubapp =
            match NubappClient::new(&cfg.app.application_id, &cfg.app.category_activity_id) {
                Ok(c) => c,
                Err(e) => {
                    users_data.push(UserDashboard {
                        name: user.name.clone(),
                        bookings: vec![],
                        waiting_list: vec![],
                        error: Some(format!("Client init failed: {e}")),
                    });
                    continue;
                }
            };

        if let Err(e) = nubapp.login(&user.login, &user.password).await {
            users_data.push(UserDashboard {
                name: user.name.clone(),
                bookings: vec![],
                waiting_list: vec![],
                error: Some(format!("Login failed: {e}")),
            });
            continue;
        }

        let resp = match nubapp.get_bookings().await {
            Ok(r) => r,
            Err(e) => {
                users_data.push(UserDashboard {
                    name: user.name.clone(),
                    bookings: vec![],
                    waiting_list: vec![],
                    error: Some(format!("Failed to fetch bookings: {e}")),
                });
                continue;
            }
        };

        let data = resp.get("data");

        // Parse bookings
        let bookings: Vec<BookingRow> = data
            .and_then(|d| d.get("bookings"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|b| BookingRow {
                        start: json_str(b, &["start_timestamp", "start"]),
                        end: json_str(b, &["end_timestamp", "end"]),
                        name: json_str(b, &["name_activity", "name"]),
                        inscribed: b
                            .get("n_inscribed")
                            .and_then(|v| v.as_u64())
                            .map(|v| v as u32),
                        capacity: b
                            .get("n_capacity")
                            .and_then(|v| v.as_u64())
                            .map(|v| v as u32),
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Parse waiting list entries
        let wl_entries: Vec<serde_json::Value> = data
            .and_then(|d| d.get("in_waiting_list"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        // Fetch slot capacity for waiting list entries
        let mut capacity_map: HashMap<String, (u32, u32)> = HashMap::new();
        if !wl_entries.is_empty() {
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
        }

        let waiting_list: Vec<WaitingRow> = wl_entries
            .iter()
            .map(|b| {
                let slot_id = b
                    .get("id_activity_calendar")
                    .map(|v| v.to_string().trim_matches('"').to_string())
                    .unwrap_or_default();
                let (ins, cap) = capacity_map.get(&slot_id).copied().unzip();
                WaitingRow {
                    start: json_str(b, &["start_timestamp", "start"]),
                    end: json_str(b, &["end_timestamp", "end"]),
                    name: json_str(b, &["name_activity", "name"]),
                    inscribed: ins,
                    capacity: cap,
                }
            })
            .collect();

        users_data.push(UserDashboard {
            name: user.name.clone(),
            bookings,
            waiting_list,
            error: None,
        });
    }

    let last_check = state.last_watcher_check.lock().unwrap().clone();
    let mut sched_entries: Vec<SchedulerEntry> = state
        .scheduler_entries
        .lock()
        .unwrap()
        .values()
        .cloned()
        .collect();
    sched_entries.sort_by(|a, b| a.target_date.cmp(&b.target_date));
    let html = render_page(cfg, &users_data, last_check, &sched_entries);
    Html(html)
}
