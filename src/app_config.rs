use std::fmt;

use config::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub enable_shutdown_endpoint: bool,
    pub max_file_size: usize,
    pub port: u16,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            enable_shutdown_endpoint: false,
            max_file_size: usize::MAX,
            port: 8000,
        }
    }
}

impl fmt::Display for AppConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            concat!(
                "\tenable_shutdown_endpoint: {}\n",
                "\tmax_file_size: {}\n",
                "\tport: {}",
            ),
            self.enable_shutdown_endpoint, self.max_file_size, self.port,
        )
    }
}

pub fn load() -> AppConfig {
    Config::builder()
        .add_source(config::File::with_name("Config"))
        .add_source(config::Environment::with_prefix("app"))
        .build()
        .unwrap_or_default()
        .try_deserialize()
        .unwrap_or_default()
}
