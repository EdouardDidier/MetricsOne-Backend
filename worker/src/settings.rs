use config::{Config, ConfigError, Environment};
use once_cell::sync::Lazy;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ServerSettings {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct ApiSettings {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub server: ServerSettings,
    pub api: ApiSettings,
    pub livetiming_url: String,
    pub rust_log: String,
}

impl Settings {
    pub fn from_env() -> Result<Self, ConfigError> {
        Config::builder()
            .add_source(Environment::default())
            .build()?
            .try_deserialize::<Settings>()
    }
}

// Loading environment variables
pub static ENV: Lazy<Settings> = Lazy::new(|| {
    dotenv::dotenv().ok();
    // Use of 'expect' here because logger is not set after loading environment variables
    Settings::from_env().expect("Failed to parse environment variables")
});
