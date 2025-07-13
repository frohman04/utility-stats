use serde::Deserialize;
use std::fs;

#[derive(Debug, Eq, PartialEq, Deserialize)]
pub struct Config {
    pub electric_file: String,
    pub gas_file: String,
    pub smoothing_days: u8,
    pub visual_crossing: VisualCrossing,
}

#[derive(Debug, Eq, PartialEq, Deserialize)]
pub struct VisualCrossing {
    pub address: String,
    pub api_key: String,
}

impl Config {
    pub fn from_file(path: &str) -> Self {
        let conf_str = fs::read_to_string(path).expect("Unable to find config file");
        let conf: Config = serde_json::from_str(&conf_str).expect("Unable to parse config");
        conf
    }
}
