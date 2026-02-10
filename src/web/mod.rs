pub mod dashboard;
pub mod slot_scheduler;
pub mod views;
pub mod watcher;

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use axum::routing::get;
use axum::Router;
use chrono::{DateTime, Local};
use tokio::net::TcpListener;
use tracing::info;

use crate::models::Config;

#[derive(Clone)]
pub(crate) struct SchedulerEntry {
    pub(crate) user_name: String,
    pub(crate) day: String,
    pub(crate) time: String,
    pub(crate) target_date: String,
    pub(crate) books_at: String,
    pub(crate) status: String,
}

pub(crate) type SchedulerState = Arc<Mutex<HashMap<String, SchedulerEntry>>>;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) config: Arc<Config>,
    pub(crate) last_watcher_check: Arc<Mutex<Option<DateTime<Local>>>>,
    pub(crate) scheduler_entries: SchedulerState,
}

pub async fn serve(config: Config, config_path: &Path, addr: &str) -> Result<()> {
    let last_check: Arc<Mutex<Option<DateTime<Local>>>> = Arc::new(Mutex::new(None));
    let scheduler_entries: SchedulerState = Arc::new(Mutex::new(HashMap::new()));
    let state_path = config_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("scheduler_state.json");
    let state = AppState {
        config: Arc::new(config),
        last_watcher_check: Arc::clone(&last_check),
        scheduler_entries: Arc::clone(&scheduler_entries),
    };

    // Spawn background watcher for waiting list auto-booking
    tokio::spawn(watcher::waiting_list_watcher(
        Arc::clone(&state.config),
        last_check,
    ));

    // Spawn slot booking schedulers for each user × configured day
    slot_scheduler::spawn_slot_schedulers(
        Arc::clone(&state.config),
        scheduler_entries,
        state_path,
    );

    let app = Router::new()
        .route("/", get(dashboard::dashboard_handler))
        .with_state(state);

    let listener = TcpListener::bind(addr).await?;
    info!("Dashboard listening on http://{}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}
