use crate::json_file_updater::{JsonFileUpdater, UpdateResult};
use serde_json::Value as JsonValue;

static VERSION_FIELD_NAME: &str = "config_format_version";
pub static LATEST_CONFIG_VERSION: &str = "0.0.2";

pub fn update_config_to_the_latest_version(
    mut config_json: JsonValue,
) -> Result<JsonValue, String> {
    let version = config_json[VERSION_FIELD_NAME].as_str();
    if let Some(version) = version {
        if version == LATEST_CONFIG_VERSION {
            return Ok(config_json);
        }
    }

    let json_config_updater = register_config_updaters();

    let update_result = json_config_updater.update_json(&mut config_json);

    return match update_result {
        UpdateResult::Error(error) => Err(error),
        _ => Ok(config_json),
    };
}

fn register_config_updaters() -> JsonFileUpdater {
    let mut json_config_updater = JsonFileUpdater::new(VERSION_FIELD_NAME);

    json_config_updater.add_update_function("0.0.1", |_config_json| {
        // empty update function to create the initial version
    });
    json_config_updater.add_update_function("0.0.2", |config_json| {
        config_json["network_interface"] = JsonValue::String("0.0.0.0".to_string());
    });

    // add update functions above this line
    // don't forget to update LATEST_CONFIG_VERSION at the beginning of the file

    json_config_updater
}
