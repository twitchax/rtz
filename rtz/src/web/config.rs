//! The configuration module.

use config::{Environment, File};
use rtz_core::base::types::Res;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct OptionalConfig {
    bind_address: Option<String>,
    port: Option<u16>,
    should_log: Option<bool>,
}

/// The configuration type.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub(crate) bind_address: String,
    pub(crate) port: u16,
    pub(crate) should_log: bool,
}

impl Config {
    /// Initializes a new [`Config`] object from the specified configuration path.
    ///
    /// Alternatively, this method will fallback to environment variables with the
    /// prefix `RTZ` (e.g., `RTZ_BIND_ADDRESS`).
    pub fn new(config_path: &str, cli_bind_address: Option<String>, cli_port: Option<u16>, cli_should_log: Option<bool>) -> Res<Self> {
        let builder = config::Config::builder()
            .add_source(File::with_name(config_path).required(false))
            .add_source(Environment::with_prefix("rtz"));

        let optional_config: OptionalConfig = builder.build()?.try_deserialize()?;

        let config = Config {
            bind_address: optional_config.bind_address.unwrap_or_else(|| {
                cli_bind_address.unwrap_or_else(|| {
                    println!("No bind address specified. Defaulting to `0.0.0.0`.");
                    "0.0.0.0".to_string()
                })
            }),
            port: optional_config.port.unwrap_or_else(|| {
                cli_port.unwrap_or_else(|| {
                    println!("No port specified. Defaulting to `8082`.");
                    8082
                })
            }),
            should_log: optional_config.should_log.unwrap_or_else(|| {
                cli_should_log.unwrap_or_else(|| {
                    println!("No logging preference specified. Defaulting to `true`.");
                    true
                })
            }),
        };

        Ok(config)
    }
}
