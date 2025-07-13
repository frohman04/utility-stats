#![forbid(unsafe_code)]

extern crate clap;
extern crate csv;
extern crate env_logger;
extern crate flate2;
#[macro_use]
extern crate log;
extern crate reqwest;
extern crate rmp_serde;
#[macro_use]
extern crate rusqlite;
extern crate serde;
extern crate time;

mod grapher;
mod measurement;
mod regression;
#[macro_use]
mod timed;
mod client;
mod config;
mod tmpmgr;

use crate::grapher::graph_all;
use crate::measurement::Measurements;
use crate::tmpmgr::TempDataManager;
use client::WeatherClient;
use client::visual_crossing::VisualCrossingClient;

use clap::{Arg, Command};
use env_logger::Env;

use crate::client::cache::ClientCache;
use crate::config::Config;
use std::path::Path;

fn main() {
    let env = Env::default().filter_or("MY_LOG_LEVEL", "info");
    env_logger::init_from_env(env);

    let matches = Command::new("utility-stats")
        .version("0.2")
        .author("Chris Lieb")
        .arg(Arg::new("config").default_value("config.json"))
        .get_matches();
    let config_file = matches.get_one::<String>("config").unwrap().as_str();
    let config = Config::from_file(config_file);

    let cache = ClientCache::new("cache".to_string());

    let client: Box<dyn WeatherClient> = Box::new(VisualCrossingClient::new(
        config.visual_crossing.address.clone(),
        config.visual_crossing.api_key.clone(),
        &cache,
    ));
    let mut mgr = TempDataManager::new(client);

    info!("Reading electric data from {}", config.electric_file);
    let electric = timed!(
        "Reading electric data from {}",
        config.electric_file,
        (|| {
            let measurements = Measurements::from_file(
                Path::new(&config.electric_file),
                "Electricity".to_string(),
                "kWh".to_string(),
            )
            .expect("Unable to read electric data");

            info!(
                "Read {} records covering {} days",
                measurements.data.len(),
                (measurements.data.last().unwrap().date - measurements.data[0].date).whole_days()
            );

            measurements
        })
    );

    let gas = timed!(
        "Reading gas data from {}",
        config.gas_file,
        (|| {
            let measurements = Measurements::from_file(
                Path::new(&config.gas_file),
                "Gas".to_string(),
                "CCF".to_string(),
            )
            .expect("Unable to read gas data");

            info!(
                "Read {} records covering {} days",
                measurements.data.len(),
                (measurements.data.last().unwrap().date - measurements.data[0].date).whole_days()
            );

            measurements
        })
    );

    timed!(
        "Drawing graph with smoothing days {}",
        config.smoothing_days,
        (|| graph_all(electric, gas, &mut mgr, config.smoothing_days))
    );
}
