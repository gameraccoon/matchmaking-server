use serde::{Deserialize, Serialize};
use std::io::Write;

use crate::config_updaters;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub config_format_version: String,
    pub working_directiries_path: String,
    pub dedicated_server_dir: String,
    pub matchmaker_port: u16,
}

pub fn read_config() -> Result<Config, String> {
    let app_arguments = std::env::args().collect::<Vec<String>>();

    let config_path = if app_arguments.len() == 2 {
        app_arguments[1].clone()
    } else {
        "data/config.json".to_string()
    };

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

pub fn generate_default_config() {
    let default_config = Config {
        working_directiries_path: "instances".to_string(),
        dedicated_server_dir: ".".to_string(),
        matchmaker_port: 14736,
        config_format_version: config_updaters::LATEST_CONFIG_VERSION.to_string(),
    };

    let default_config_json = serde_json::to_string_pretty(&default_config).unwrap();

    let config_path = "config.json";
    let mut file = std::fs::File::create(config_path).unwrap();
    file.write_all(default_config_json.as_bytes()).unwrap();
}
