use serde::Deserialize;
use std::fs;

#[derive(Debug, Eq, PartialEq, Deserialize)]
pub struct Config {
    pub address: String,
    pub visual_crossing_api_key: String,
    pub electric_file: String,
    pub gas_file: String,
    pub smoothing_days: u8,
}

impl Config {
    pub fn from_file(path: &str) -> Self {
        let conf_str = fs::read_to_string(path).expect("Unable to find config file");
        let conf: Config = serde_json::from_str(&conf_str).expect("Unable to parse config");
        conf
    }
}
