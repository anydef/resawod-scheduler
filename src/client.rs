use anyhow::{Context, Result};
use base64::prelude::*;
use reqwest::header::{HeaderMap, HeaderValue, ORIGIN, REFERER, USER_AGENT};
use reqwest::Client;
use tracing::{debug, info};

use crate::models::Slot;

const API_BASE: &str = "https://sport.nubapp.com/api/v4";
const BOX_ORIGIN: &str = "https://box.resawod.com";
const APP_VERSION: &str = "5.13.06";
const BROWSER_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:147.0) \
    Gecko/20100101 Firefox/147.0";

pub struct NubappClient {
    client: Client,
    application_id: String,
    category_activity_id: String,
    token: Option<String>,
    id_user: Option<String>,
}

impl NubappClient {
    pub fn new(application_id: &str, category_activity_id: &str) -> Result<Self> {
        let client = Client::builder()
            .cookie_store(true)
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self {
            client,
            application_id: application_id.to_string(),
            category_activity_id: category_activity_id.to_string(),
            token: None,
            id_user: None,
        })
    }

    fn default_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(BROWSER_UA));
        headers.insert(
            "Accept",
            HeaderValue::from_static("application/json, text/plain, */*"),
        );
        headers.insert(
            "Content-Type",
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
        headers.insert(ORIGIN, HeaderValue::from_static(BOX_ORIGIN));
        headers.insert(REFERER, HeaderValue::from_static("https://box.resawod.com/"));
        headers.insert("Nubapp-Origin", HeaderValue::from_static("user_apps"));
        headers.insert("sec-fetch-dest", HeaderValue::from_static("empty"));
        headers.insert("sec-fetch-mode", HeaderValue::from_static("cors"));
        headers.insert("sec-fetch-site", HeaderValue::from_static("cross-site"));
        if let Some(ref token) = self.token {
            if let Ok(val) = HeaderValue::from_str(&format!("Bearer {}", token)) {
                headers.insert("Authorization", val);
            }
        }
        headers
    }

    fn id_user(&self) -> Result<&str> {
        self.id_user
            .as_deref()
            .context("No id_user available — login first")
    }

    /// Authenticate the user and store the auth token + id_user
    pub async fn login(&mut self, username: &str, password: &str) -> Result<serde_json::Value> {
        let url = format!("{}/login", API_BASE);

        let resp = self
            .client
            .post(&url)
            .headers(self.default_headers())
            .body(format!(
                "username={}&password={}",
                urlencoding::encode(username),
                urlencoding::encode(password)
            ))
            .send()
            .await
            .context("Failed to send login request")?;

        let status = resp.status();
        let text = resp.text().await.context("Failed to read login response")?;
        debug!("Login response (status {}): {}", status, text);

        let body: serde_json::Value = serde_json::from_str(&text)
            .with_context(|| format!("Failed to parse login response (status {status}): {text}"))?;

        // Extract auth token and decode JWT for id_user
        let token_str = body
            .get("token")
            .or_else(|| body.get("data").and_then(|d| d.get("token")))
            .and_then(|t| t.as_str());

        if let Some(token) = token_str {
            self.token = Some(token.to_string());

            let parts: Vec<&str> = token.split('.').collect();
            if parts.len() >= 2 {
                if let Ok(payload_bytes) = BASE64_URL_SAFE_NO_PAD.decode(parts[1]) {
                    if let Ok(payload) =
                        serde_json::from_slice::<serde_json::Value>(&payload_bytes)
                    {
                        if let Some(id) = payload.get("id_user") {
                            self.id_user = Some(id.to_string());
                        }
                    }
                }
            }
            info!(
                "Logged in successfully (id_user: {:?})",
                self.id_user
            );
        } else {
            info!("Logged in (no token found in response)");
        }

        Ok(body)
    }

    /// Fetch activity categories for the gym
    pub async fn get_categories(&self) -> Result<serde_json::Value> {
        let url = format!("{}/categories/getCategories.php", API_BASE);

        let resp = self
            .client
            .post(&url)
            .headers(self.default_headers())
            .body(format!(
                "app_version={}&id_application={}",
                APP_VERSION, self.application_id
            ))
            .send()
            .await
            .context("Failed to fetch categories")?;

        let status = resp.status();
        let text = resp.text().await.context("Failed to read categories response")?;
        debug!("Categories response (status {}): {}", status, text);

        let body: serde_json::Value = serde_json::from_str(&text)
            .with_context(|| format!("Failed to parse categories (status {status}): {text}"))?;
        Ok(body)
    }

    /// Fetch available slots for a given date (format: DD-MM-YYYY)
    pub async fn get_slots(&self, date: &str) -> Result<Vec<Slot>> {
        let url = format!("{}/activities/getActivitiesCalendar.php", API_BASE);
        let id_user = self.id_user()?;

        let resp = self
            .client
            .post(&url)
            .headers(self.default_headers())
            .body(format!(
                "app_version={}&id_application={}&start_timestamp={}&end_timestamp={}&id_user={}&id_category_activity={}",
                APP_VERSION,
                self.application_id,
                date,
                date,
                id_user,
                self.category_activity_id,
            ))
            .send()
            .await
            .context("Failed to fetch slots")?;

        let status = resp.status();
        let text = resp.text().await.context("Failed to read slots response")?;
        debug!("Slots response (status {}): {}", status, text);

        // Response is wrapped in {"data": {"DD-MM-YYYY": [...]}, "success": true}
        let body: serde_json::Value = serde_json::from_str(&text)
            .with_context(|| format!("Failed to parse slots (status {status}): {text}"))?;

        let data = body.get("data").unwrap_or(&body);

        // data is {"activities_calendar": [...]} or a direct array
        let slots_value = if let Some(obj) = data.as_object() {
            obj.get("activities_calendar")
                .cloned()
                .or_else(|| obj.values().next().cloned())
                .unwrap_or(serde_json::Value::Array(vec![]))
        } else {
            data.clone()
        };

        let slots: Vec<Slot> = serde_json::from_value(slots_value.clone())
            .with_context(|| format!("Failed to parse slots array from: {}", slots_value))?;

        debug!("Fetched {} slots", slots.len());
        Ok(slots)
    }

    /// Book a specific slot
    pub async fn book(&self, id_activity_calendar: &str) -> Result<serde_json::Value> {
        let url = format!("{}/activities/bookActivityCalendar.php", API_BASE);
        let id_user = self.id_user()?;

        let resp = self
            .client
            .post(&url)
            .headers(self.default_headers())
            .body(format!(
                "app_version={}&id_application={}&id_activity_calendar={}&id_user={}&action_by={}&n_guests=0&booked_on=3",
                APP_VERSION,
                self.application_id,
                id_activity_calendar,
                id_user,
                id_user,
            ))
            .send()
            .await
            .context("Failed to send booking request")?;

        let status = resp.status();
        let text = resp.text().await.context("Failed to read booking response")?;
        debug!("Booking response (status {}): {}", status, text);

        let body: serde_json::Value = serde_json::from_str(&text)
            .with_context(|| format!("Failed to parse booking response (status {status}): {text}"))?;
        Ok(body)
    }

    /// Join waiting list for a slot
    pub async fn book_waiting_list(&self, id_activity_calendar: &str) -> Result<serde_json::Value> {
        let url = format!("{}/activities/bookWaitingActivityCalendar.php", API_BASE);
        let id_user = self.id_user()?;

        let resp = self
            .client
            .post(&url)
            .headers(self.default_headers())
            .body(format!(
                "app_version={}&id_application={}&id_activity_calendar={}&id_user={}&action_by={}",
                APP_VERSION,
                self.application_id,
                id_activity_calendar,
                id_user,
                id_user,
            ))
            .send()
            .await
            .context("Failed to send waiting list request")?;

        let status = resp.status();
        let text = resp.text().await.context("Failed to read waiting list response")?;
        debug!("Waiting list response (status {}): {}", status, text);

        let body: serde_json::Value = serde_json::from_str(&text)
            .with_context(|| format!("Failed to parse waiting list response (status {status}): {text}"))?;
        Ok(body)
    }

    /// Fetch user's future bookings
    pub async fn get_bookings(&self) -> Result<serde_json::Value> {
        let url = format!("{}/users/getUserFutureBookings.php", API_BASE);
        let id_user = self.id_user()?;

        let resp = self
            .client
            .post(&url)
            .headers(self.default_headers())
            .body(format!(
                "app_version={}&id_application={}&id_user={}&limit=50&include_waiting_list=true",
                APP_VERSION,
                self.application_id,
                id_user,
            ))
            .send()
            .await
            .context("Failed to fetch bookings")?;

        let status = resp.status();
        let text = resp.text().await.context("Failed to read bookings response")?;
        debug!("Bookings response (status {}): {}", status, text);

        let body: serde_json::Value = serde_json::from_str(&text)
            .with_context(|| format!("Failed to parse bookings (status {status}): {text}"))?;
        Ok(body)
    }

    /// Find a slot matching time and optionally activity name (partial, case-insensitive).
    /// If `activity` is empty or None, matches any slot at the given time.
    pub fn find_slot<'a>(slots: &'a [Slot], time: &str, activity: Option<&str>) -> Option<&'a Slot> {
        slots.iter().find(|s| {
            if !s.start.contains(time) {
                return false;
            }
            match activity.filter(|a| !a.is_empty()) {
                Some(a) => s
                    .name
                    .as_deref()
                    .map_or(false, |n| n.to_lowercase().contains(&a.to_lowercase())),
                None => true,
            }
        })
    }
}
