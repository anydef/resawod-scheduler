use std::path::Path;

use anyhow::Result;
use base64::prelude::*;
use chrono::Local;
use tracing::{info, warn};

use crate::client::NubappClient;
use crate::config;
use crate::models::{SlotConfig, User};
use crate::scheduler;

/// Resolve login/password from CLI flags or first user in config
pub fn resolve_credentials<'a>(
    user_flag: &'a Option<String>,
    pass_flag: &'a Option<String>,
    first_user: Option<&'a User>,
) -> Result<(&'a str, &'a str)> {
    let login = match user_flag {
        Some(u) => u.as_str(),
        None => first_user
            .map(|u| u.login.as_str())
            .ok_or_else(|| anyhow::anyhow!("No users in config and no --user provided"))?,
    };
    let pass = match pass_flag {
        Some(p) => p.as_str(),
        None => first_user
            .map(|u| u.password.as_str())
            .ok_or_else(|| anyhow::anyhow!("No users in config and no --password provided"))?,
    };
    Ok((login, pass))
}

pub async fn run_for_user(
    application_id: &str,
    category_activity_id: &str,
    verbose: bool,
    debug: bool,
    user: &User,
    slot_configs: &std::collections::HashMap<String, SlotConfig>,
) -> Result<()> {
    info!("Processing user: {}", user.name);

    let mut nubapp = NubappClient::new(application_id, category_activity_id)?;

    let login_resp = nubapp.login(&user.login, &user.password).await?;
    if verbose {
        println!(
            "Login response: {}",
            serde_json::to_string_pretty(&login_resp)?
        );
    }

    let today = Local::now().date_naive();
    let mut calendar: Vec<(String, String)> = Vec::new(); // (day, slot_id)

    for day_name in &user.slots {
        let weekday = match scheduler::parse_weekday(day_name) {
            Some(wd) => wd,
            None => {
                warn!("Unknown day '{}', skipping", day_name);
                continue;
            }
        };

        let slot_cfg = match slot_configs.get(day_name.to_lowercase().as_str()) {
            Some(c) => c,
            None => {
                warn!("No slot configured for '{}', skipping", day_name);
                continue;
            }
        };

        let target_date = scheduler::next_weekday(today, weekday);
        let date_str = target_date.format("%d-%m-%Y").to_string();

        info!(
            "{}: looking for slot at {} ({}) on {} ({})",
            user.name,
            slot_cfg.time,
            slot_cfg.activity.as_deref().unwrap_or("any"),
            target_date,
            date_str
        );

        let slots = nubapp.get_slots(&date_str).await?;

        if verbose {
            for slot in &slots {
                println!(
                    "  Available: {} - {} — {} (ID: {})",
                    slot.start,
                    slot.end,
                    slot.name.as_deref().unwrap_or("?"),
                    slot.id_activity_calendar
                );
            }
        }

        match NubappClient::find_slot(&slots, &slot_cfg.time, slot_cfg.activity.as_deref()) {
            Some(slot) => {
                let slot_id = slot.id_activity_calendar.to_string();
                let slot_id = slot_id.trim_matches('"').to_string();
                info!(
                    "Found slot: {} — {} (ID: {})",
                    slot.start,
                    slot.name.as_deref().unwrap_or("?"),
                    slot_id,
                );
                calendar.push((day_name.clone(), slot_id));
            }
            None => {
                warn!(
                    "No slot found for {} at {} ({}) on {}",
                    user.name,
                    slot_cfg.time,
                    slot_cfg.activity.as_deref().unwrap_or("any"),
                    target_date
                );
            }
        }
    }

    for (day, slot_id) in &calendar {
        if debug {
            println!(
                "[DRY RUN] Would book {} for {} (slot ID: {})",
                day, user.name, slot_id
            );
        } else {
            info!("Booking {} for {} (slot ID: {})", day, user.name, slot_id);
            let resp = nubapp.book(slot_id).await?;
            let success = resp
                .get("success")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if verbose {
                println!(
                    "Booking response: {}",
                    serde_json::to_string_pretty(&resp)?
                );
            }
            if success {
                println!("Booked {} for {}", day, user.name);
            } else {
                let msg = resp
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown error");
                warn!("Failed to book {} for {}: {}", day, user.name, msg);
                // Try waiting list
                info!("Trying waiting list for {} ...", day);
                let wl_resp = nubapp.book_waiting_list(slot_id).await?;
                let wl_success = wl_resp
                    .get("success")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if verbose {
                    println!(
                        "Waiting list response: {}",
                        serde_json::to_string_pretty(&wl_resp)?
                    );
                }
                if wl_success {
                    println!("Added to waiting list for {} for {}", day, user.name);
                } else {
                    let wl_msg = wl_resp
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown error");
                    warn!(
                        "Failed to join waiting list for {} for {}: {}",
                        day, user.name, wl_msg
                    );
                }
            }
        }
    }

    if calendar.is_empty() {
        println!("No slots to book for {}", user.name);
    }

    Ok(())
}

pub async fn run_discover(
    application_id: &str,
    username: &str,
    password: &str,
    verbose: bool,
) -> Result<()> {
    let mut nubapp = NubappClient::new(application_id, "0")?;

    println!("Logging in as {}...", username);
    let login_resp = nubapp.login(username, password).await?;

    // Decode JWT token to extract account info
    if let Some(token) = login_resp.get("token").and_then(|t| t.as_str()) {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() >= 2 {
            if let Ok(payload_bytes) = BASE64_URL_SAFE_NO_PAD.decode(parts[1]) {
                if let Ok(payload) =
                    serde_json::from_slice::<serde_json::Value>(&payload_bytes)
                {
                    println!("\n=== Account Information ===");
                    if let Some(id) = payload.get("id_application") {
                        println!("  application_id: {}", id);
                    }
                    if let Some(id) = payload.get("id_user") {
                        println!("  user_id:        {}", id);
                    }
                    if let Some(name) = payload.get("username") {
                        println!("  username:       {}", name.as_str().unwrap_or("?"));
                    }
                    if verbose {
                        println!(
                            "\n  Full JWT payload:\n  {}",
                            serde_json::to_string_pretty(&payload)?
                        );
                    }
                }
            }
        }
    }

    // Fetch activity categories
    println!("\n=== Activity Categories ===");
    match nubapp.get_categories().await {
        Ok(resp) => {
            let cats = resp.get("data").unwrap_or(&resp);
            match cats {
                serde_json::Value::Array(arr) if !arr.is_empty() => {
                    for cat in arr {
                        let id = cat
                            .get("id_category_activity")
                            .or_else(|| cat.get("id"))
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| "?".into());
                        let name = cat
                            .get("name")
                            .or_else(|| cat.get("title"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        println!("  [{}] {}", id.trim_matches('"'), name);
                    }
                }
                other => {
                    if verbose {
                        println!(
                            "  Raw response:\n  {}",
                            serde_json::to_string_pretty(other)?
                        );
                    } else {
                        println!("  Could not list categories. Re-run with -v for details.");
                    }
                }
            }
        }
        Err(e) => {
            println!("  Could not fetch categories: {}", e);
        }
    }

    println!("\nUse these values in your config.toml under [app].");
    Ok(())
}

fn print_booking(b: &serde_json::Value) {
    let start = b
        .get("start_timestamp")
        .or_else(|| b.get("start"))
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let end = b
        .get("end_timestamp")
        .or_else(|| b.get("end"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let name = b
        .get("name")
        .or_else(|| b.get("name_activity"))
        .and_then(|v| v.as_str())
        .unwrap_or("?")
        .trim();
    let spots = b.get("n_inscribed").and_then(|v| v.as_u64());
    let capacity = b.get("n_capacity").and_then(|v| v.as_u64());

    print!("  {} to {} — {}", start, end, name);
    if let (Some(s), Some(c)) = (spots, capacity) {
        print!(" ({}/{})", s, c);
    }
    println!();
}

fn print_waiting_list_entry(b: &serde_json::Value) {
    let start = b
        .get("start_timestamp")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let end = b
        .get("end_timestamp")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let name = b
        .get("name_activity")
        .or_else(|| b.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("?")
        .trim();

    print!("  {} to {} — {}", start, end, name);
    let inscribed = b.get("n_inscribed").and_then(|v| v.as_u64());
    let capacity = b.get("n_capacity").and_then(|v| v.as_u64());
    if let (Some(s), Some(c)) = (inscribed, capacity) {
        let free = c.saturating_sub(s);
        print!(" ({}/{}, {} free)", s, c, free);
    }
    println!();
}

pub async fn run_bookings(
    verbose: bool,
    config_path: &Path,
    user: &Option<String>,
    password: &Option<String>,
) -> Result<()> {
    let cfg = config::load_config(config_path)?;
    let (login, pass) = resolve_credentials(user, password, cfg.users.first())?;

    let mut nubapp =
        NubappClient::new(&cfg.app.application_id, &cfg.app.category_activity_id)?;
    nubapp.login(login, pass).await?;

    let resp = nubapp.get_bookings().await?;

    if verbose {
        println!("{}", serde_json::to_string_pretty(&resp)?);
        return Ok(());
    }

    let data = resp.get("data");

    match data
        .and_then(|d| d.get("bookings"))
        .and_then(|v| v.as_array())
    {
        Some(arr) if !arr.is_empty() => {
            println!("Bookings for {}:\n", login);
            for b in arr {
                print_booking(b);
            }
        }
        _ => println!("No upcoming bookings for {}.", login),
    }

    if let Some(arr) = data
        .and_then(|d| d.get("in_waiting_list"))
        .and_then(|v| v.as_array())
    {
        if !arr.is_empty() {
            // Collect unique dates to fetch slot capacity
            let mut dates: Vec<String> = arr
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

            // Fetch slots for each date and build a lookup by id_activity_calendar
            let mut capacity_map: std::collections::HashMap<String, (u64, u64)> =
                std::collections::HashMap::new();
            for date in &dates {
                // Convert YYYY-MM-DD to DD-MM-YYYY for the API
                if let Some(api_date) = date
                    .get(8..10)
                    .zip(date.get(5..7))
                    .zip(date.get(0..4))
                    .map(|((d, m), y)| format!("{}-{}-{}", d, m, y))
                {
                    if let Ok(slots) = nubapp.get_slots(&api_date).await {
                        for slot in &slots {
                            let id = slot.id_activity_calendar.to_string();
                            let id = id.trim_matches('"').to_string();
                            if let (Some(ins), Some(cap)) = (slot.n_inscribed, slot.n_capacity) {
                                capacity_map.insert(id, (ins as u64, cap as u64));
                            }
                        }
                    }
                }
            }

            println!("\nWaiting list:\n");
            for b in arr {
                print_waiting_list_entry(b);
                // Look up capacity by id_activity_calendar
                if let Some(id) = b.get("id_activity_calendar") {
                    let id_str = id.to_string();
                    let id_str = id_str.trim_matches('"');
                    if let Some(&(ins, cap)) = capacity_map.get(id_str) {
                        let free = cap.saturating_sub(ins);
                        println!("    ^ {}/{} booked, {} free", ins, cap, free);
                    }
                }
            }
        }
    }

    Ok(())
}
