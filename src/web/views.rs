use std::collections::HashMap;

use chrono::DateTime;
use chrono_tz::Tz;
use leptos::prelude::*;

use super::dashboard::{BookingRow, UserDashboard, WaitingRow};
use super::SchedulerEntry;
use crate::models::{self, Config};

const STYLE: &str = include_str!("../style.css");

pub(super) fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + c.as_str(),
    }
}

pub(super) fn render_page(
    cfg: &Config,
    users: &[UserDashboard],
    last_watcher_check: Option<DateTime<Tz>>,
    scheduler_entries: &[SchedulerEntry],
) -> String {
    let slots_html = render_slots_table(&cfg.slots);
    let scheduler_html = render_scheduler_table(scheduler_entries);
    let users_html: String = users.iter().map(render_user_section).collect();
    let now = crate::scheduler::now().format("%Y-%m-%d %H:%M:%S %Z").to_string();
    let watcher_status = match last_watcher_check {
        Some(t) => format!("Last watcher check: {}", t.format("%Y-%m-%d %H:%M:%S")),
        None => "Watcher: waiting for first check...".to_string(),
    };

    view! {
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <title>"RESAWOD Dashboard"</title>
                <style>{STYLE}</style>
            </head>
            <body>
                <h1>"RESAWOD Dashboard"</h1>
                <p class="timestamp">"Updated: " {now}</p>
                <p class="watcher-status">{watcher_status}</p>
                <section>
                    <h2>"Configured Slots"</h2>
                    <div inner_html=slots_html />
                </section>
                <section>
                    <h2>"Scheduled Bookings"</h2>
                    <div inner_html=scheduler_html />
                </section>
                <div inner_html=users_html />
            </body>
        </html>
    }
    .to_html()
}

fn render_slots_table(slots: &HashMap<String, models::SlotConfig>) -> String {
    if slots.is_empty() {
        return view! { <p class="empty">"No slots configured."</p> }.to_html();
    }

    let days = [
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
    ];
    let rows: Vec<(String, String, String)> = days
        .iter()
        .filter_map(|d| {
            slots
                .get(*d)
                .map(|c| (capitalize(d), c.time.clone(), c.activity.clone().unwrap_or_default()))
        })
        .collect();

    let rows_html: String = rows
        .iter()
        .map(|(day, time, activity)| {
            let day = day.clone();
            let time = time.clone();
            let activity = activity.clone();
            view! {
                <tr>
                    <td>{day}</td>
                    <td>{time}</td>
                    <td>{activity}</td>
                </tr>
            }
            .to_html()
        })
        .collect();

    view! {
        <table>
            <thead>
                <tr><th>"Day"</th><th>"Time"</th><th>"Activity"</th></tr>
            </thead>
            <tbody inner_html=rows_html />
        </table>
    }
    .to_html()
}

fn render_user_section(user: &UserDashboard) -> String {
    let name = user.name.clone();

    if let Some(ref err) = user.error {
        let err = err.clone();
        return view! {
            <section>
                <h2>{name}</h2>
                <div class="error">{err}</div>
            </section>
        }
        .to_html();
    }

    let bookings_html = render_bookings_table(&user.bookings);
    let waiting_html = render_waiting_table(&user.waiting_list);

    view! {
        <section>
            <h2>{name}</h2>
            <h3>"Bookings"</h3>
            <div inner_html=bookings_html />
            <h3>"Waiting List"</h3>
            <div inner_html=waiting_html />
        </section>
    }
    .to_html()
}

fn render_bookings_table(bookings: &[BookingRow]) -> String {
    if bookings.is_empty() {
        return view! { <p class="empty">"No upcoming bookings."</p> }.to_html();
    }

    let rows_html: String = bookings
        .iter()
        .map(|b| {
            let capacity_text = match (b.inscribed, b.capacity) {
                (Some(i), Some(c)) => format!("{i}/{c}"),
                _ => String::new(),
            };
            let start = b.start.clone();
            let end = b.end.clone();
            let name = b.name.clone();

            view! {
                <tr>
                    <td>{start}</td>
                    <td>{end}</td>
                    <td>{name}</td>
                    <td class="capacity">{capacity_text}</td>
                </tr>
            }
            .to_html()
        })
        .collect();

    view! {
        <table>
            <thead>
                <tr><th>"Start"</th><th>"End"</th><th>"Activity"</th><th>"Capacity"</th></tr>
            </thead>
            <tbody inner_html=rows_html />
        </table>
    }
    .to_html()
}

fn render_waiting_table(entries: &[WaitingRow]) -> String {
    if entries.is_empty() {
        return view! { <p class="empty">"Not on any waiting lists."</p> }.to_html();
    }

    let rows_html: String = entries
        .iter()
        .map(|w| {
            let (capacity_text, css) = match (w.inscribed, w.capacity) {
                (Some(i), Some(c)) => {
                    let free = c.saturating_sub(i);
                    let class = if free == 0 {
                        "capacity full"
                    } else {
                        "capacity available"
                    };
                    (format!("{i}/{c} ({free} free)"), class)
                }
                _ => (String::new(), "capacity"),
            };
            let start = w.start.clone();
            let end = w.end.clone();
            let name = w.name.clone();
            let css = css.to_string();

            view! {
                <tr>
                    <td>{start}</td>
                    <td>{end}</td>
                    <td>{name}</td>
                    <td class=css>{capacity_text}</td>
                </tr>
            }
            .to_html()
        })
        .collect();

    view! {
        <table>
            <thead>
                <tr><th>"Start"</th><th>"End"</th><th>"Activity"</th><th>"Capacity"</th></tr>
            </thead>
            <tbody inner_html=rows_html />
        </table>
    }
    .to_html()
}

fn render_scheduler_table(entries: &[SchedulerEntry]) -> String {
    if entries.is_empty() {
        return view! { <p class="empty">"No scheduled bookings yet."</p> }.to_html();
    }

    let rows_html: String = entries
        .iter()
        .map(|e| {
            let user = e.user_name.clone();
            let slot = format!("{} {}", e.day, e.time);
            let target = e.target_date.clone();
            let books_at = e.books_at.clone();
            let status = e.status.clone();
            let css = match status.as_str() {
                "booked" | "already booked" => "status-booked",
                s if s.starts_with("error") || s.starts_with("failed") => "status-error",
                "booking..." => "status-active",
                _ => "status-pending",
            }
            .to_string();

            view! {
                <tr>
                    <td>{user}</td>
                    <td>{slot}</td>
                    <td>{target}</td>
                    <td>{books_at}</td>
                    <td class=css>{status}</td>
                </tr>
            }
            .to_html()
        })
        .collect();

    view! {
        <table>
            <thead>
                <tr>
                    <th>"User"</th>
                    <th>"Slot"</th>
                    <th>"Target Date"</th>
                    <th>"Books At"</th>
                    <th>"Status"</th>
                </tr>
            </thead>
            <tbody inner_html=rows_html />
        </table>
    }
    .to_html()
}
