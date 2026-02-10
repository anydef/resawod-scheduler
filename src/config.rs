use std::path::Path;

use anyhow::{Context, Result};

use crate::models::Config;

pub fn load_config(path: &Path) -> Result<Config> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let config: Config =
        toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(config)
}
