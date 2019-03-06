extern crate clap;
extern crate csv;
#[macro_use]
extern crate log;
extern crate reqwest;
extern crate rmp_serde;
#[macro_use]
extern crate serde_derive;
extern crate simplelog;

mod darksky;
mod grapher;
mod measurement;
mod regression;
#[macro_use]
mod timed;

use darksky::DarkSkyClient;
use grapher::graph_all;
use measurement::Measurements;

use chrono::prelude::*;
use clap::{App, Arg};
use simplelog::{CombinedLogger, Config, LevelFilter, TermLogger};

use std::path::Path;

fn main() -> () {
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Info, Config::default()).unwrap()
    ])
    .unwrap();

    let matches = App::new("utility-stats")
        .version("0.1")
        .author("Chris Lieb")
        .arg(
            Arg::with_name("smoothing_days")
                .short("s")
                .long("smoothing_days")
                .default_value("14"),
        )
        .arg(
            Arg::with_name("electric_file")
                .short("e")
                .long("electric_file")
                .default_value("electric.csv"),
        )
        .arg(
            Arg::with_name("gas_file")
                .short("g")
                .long("gas_file")
                .default_value("gas.csv"),
        )
        .get_matches();
    let electric_file = matches.value_of("electric_file").unwrap();
    let gas_file = matches.value_of("gas_file").unwrap();
    let smoothing_days = matches
        .value_of("smoothing_days")
        .unwrap()
        .parse::<u8>()
        .unwrap();

    let client = DarkSkyClient::new("9fff3709265bf41d21854d403ed7ee98".to_string());
    let response = client.get_history(Utc.ymd(2019, 3, 1));
    println!("{:?}", response);

    info!("Reading electric data from {}", electric_file);
    let electric = timed!(
        "Reading electric data from {}",
        electric_file,
        (|| {
            let measurements = Measurements::from_file(
                Path::new(electric_file),
                "Electricity".to_string(),
                "kWh".to_string(),
            )
            .expect("Unable to read electric data");

            info!(
                "Read {} records covering {} days",
                measurements.data.len(),
                measurements
                    .data
                    .last()
                    .unwrap()
                    .date
                    .signed_duration_since(measurements.data[0].date)
                    .num_days()
            );

            measurements
        })
    );

    let gas = timed!(
        "Reading gas data from {}",
        gas_file,
        (|| {
            let measurements =
                Measurements::from_file(Path::new(gas_file), "Gas".to_string(), "CCF".to_string())
                    .expect("Unable to read gas data");

            info!(
                "Read {} records covering {} days",
                measurements.data.len(),
                measurements
                    .data
                    .last()
                    .unwrap()
                    .date
                    .signed_duration_since(measurements.data[0].date)
                    .num_days()
            );

            measurements
        })
    );

    timed!(
        "Drawing graph with smoothing days {}",
        smoothing_days,
        (|| graph_all(electric, gas, smoothing_days))
    );
}
