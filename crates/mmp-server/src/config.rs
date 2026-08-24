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
mod tests {
    use super::*;

    #[test]
    fn the_default_port_avoids_the_crowded_ones() {
        let address: SocketAddr = DEFAULT_BIND_ADDRESS.parse().unwrap();
        let port = address.port();
        assert_eq!(port, 7979);
        for crowded in [80, 3000, 5000, 8000, 8080, 8081, 8888, 9000] {
            assert_ne!(port, crowded, "{crowded} is too commonly used");
        }
    }

    #[test]
    fn flags_accept_the_usual_spellings() {
        unsafe { env::set_var("MMP_TEST_FLAG", "TRUE") };
        assert!(flag("MMP_TEST_FLAG", false).unwrap());
        unsafe { env::set_var("MMP_TEST_FLAG", "off") };
        assert!(!flag("MMP_TEST_FLAG", true).unwrap());
        unsafe { env::remove_var("MMP_TEST_FLAG") };
        assert!(flag("MMP_TEST_FLAG", true).unwrap());
    }

    #[test]
    fn a_nonsense_flag_is_rejected() {
        unsafe { env::set_var("MMP_TEST_BAD_FLAG", "perhaps") };
        assert!(flag("MMP_TEST_BAD_FLAG", true).is_err());
        unsafe { env::remove_var("MMP_TEST_BAD_FLAG") };
    }
}
