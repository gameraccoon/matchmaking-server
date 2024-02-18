use serde::{Deserialize, Serialize};
use std::io::Write;

use crate::config_updaters;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub config_format_version: String,
    pub working_directiries_path: String,
    pub dedicated_server_dir: String,
    pub network_interface: String,
    pub matchmaker_port: u16,
}

pub fn read_config(config_path: &str) -> Result<Config, String> {
    let data = std::fs::read_to_string(&config_path);
    let data = match data {
        Ok(data) => data,
        Err(error) => return Err(error.to_string()),
    };

    let config_json = serde_json::from_str(&data);
    let config_json = match config_json {
        Ok(config_json) => config_json,
        Err(error) => return Err(error.to_string()),
    };

    let config_json = config_updaters::update_config_to_the_latest_version(config_json);
    let config_json = match config_json {
        Ok(config_json) => config_json,
        Err(error) => return Err(error),
    };

    let config = serde_json::from_value(config_json);
    let config = match config {
        Ok(config) => config,
        Err(error) => return Err(error.to_string()),
    };

    return Ok(config);
}

pub fn generate_default_config(config_path: &str) {
    let default_config = Config {
        working_directiries_path: "instances".to_string(),
        dedicated_server_dir: ".".to_string(),
        network_interface: "0.0.0.0".to_string(),
        matchmaker_port: 14736,
        config_format_version: config_updaters::LATEST_CONFIG_VERSION.to_string(),
    };

    let default_config_json = serde_json::to_string_pretty(&default_config).unwrap();

    let config_dir = std::path::Path::new(config_path).parent().unwrap();
    std::fs::create_dir_all(config_dir).unwrap();

    let mut file = std::fs::File::create(config_path).unwrap();
    file.write_all(default_config_json.as_bytes()).unwrap();
}
