use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub app: AppConfig,
    pub users: Vec<User>,
    pub slots: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AppConfig {
    pub application_id: String,
    pub category_activity_id: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct User {
    pub name: String,
    pub login: String,
    pub password: String,
    pub slots: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Slot {
    #[serde(alias = "start_timestamp", alias = "start")]
    pub start: String,
    #[serde(alias = "end_timestamp", alias = "end")]
    pub end: String,
    pub id_activity_calendar: serde_json::Value,
    #[serde(default, alias = "name_activity")]
    pub name: Option<String>,
    #[serde(default)]
    pub n_inscribed: Option<u32>,
    #[serde(default)]
    pub n_capacity: Option<u32>,
}
