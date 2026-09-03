use std::env;
use std::net::SocketAddr;

pub const DEFAULT_BIND_ADDRESS: &str = "0.0.0.0:7979";

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_address: SocketAddr,
    pub database_url: String,
    pub dev_user: String,
    pub dev_password: String,
    pub web_dist: Option<String>,
    pub seed_on_start: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{0} must be set")]
    Missing(&'static str),
    #[error("{name} is not valid: {reason}")]
    Invalid { name: &'static str, reason: String },
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let bind_address = optional("MMP_BIND_ADDRESS")
            .unwrap_or_else(|| DEFAULT_BIND_ADDRESS.to_owned())
            .parse()
            .map_err(|e: std::net::AddrParseError| ConfigError::Invalid {
                name: "MMP_BIND_ADDRESS",
                reason: e.to_string(),
            })?;

        Ok(Self {
            bind_address,
            database_url: required("DATABASE_URL")?,
            dev_user: optional("MMP_DEV_USER").unwrap_or_else(|| "admin".to_owned()),
            dev_password: required("MMP_DEV_PASSWORD")?,
            web_dist: optional("MMP_WEB_DIST"),
            seed_on_start: flag("MMP_SEED_ON_START", true)?,
        })
    }
}

fn required(name: &'static str) -> Result<String, ConfigError> {
    optional(name).ok_or(ConfigError::Missing(name))
}

fn flag(name: &'static str, default: bool) -> Result<bool, ConfigError> {
    match optional(name) {
        None => Ok(default),
        Some(raw) => match raw.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" => Ok(false),
            other => Err(ConfigError::Invalid {
                name,
                reason: format!("expected a boolean, got `{other}`"),
            }),
        },
    }
}

fn optional(name: &str) -> Option<String> {
    env::var(name).ok().filter(|v| !v.trim().is_empty())
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
