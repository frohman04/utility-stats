#![forbid(unsafe_code)]

extern crate clap;
extern crate csv;
extern crate flate2;
#[macro_use]
extern crate log;
extern crate reqwest;
extern crate rmp_serde;
#[macro_use]
extern crate rusqlite;
extern crate serde;
extern crate simplelog;
extern crate time;

mod darksky;
mod grapher;
mod measurement;
mod regression;
#[macro_use]
mod timed;
mod tmpmgr;
mod visual_crossing;
mod weatherclient;

use crate::darksky::DarkSkyClient;
use crate::grapher::graph_all;
use crate::measurement::Measurements;
use crate::tmpmgr::TempDataManager;
use crate::visual_crossing::VisualCrossingClient;
use crate::weatherclient::WeatherClient;

use clap::{Arg, Command};
use simplelog::{ColorChoice, CombinedLogger, Config, LevelFilter, TermLogger, TerminalMode};

use std::path::Path;

fn main() {
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Stderr,
        ColorChoice::Auto,
    )])
    .unwrap();

    let matches = Command::new("utility-stats")
        .version("0.1")
        .author("Chris Lieb")
        .arg(
            Arg::new("smoothing_days")
                .short('s')
                .long("smoothing_days")
                .default_value("14"),
        )
        .arg(
            Arg::new("electric_file")
                .short('e')
                .long("electric_file")
                .default_value("electric.csv"),
        )
        .arg(
            Arg::new("gas_file")
                .short('g')
                .long("gas_file")
                .default_value("gas.csv"),
        )
        .arg(
            Arg::new("visual_crossing")
                .long("vc")
                .takes_value(false)
                .help("Use VisualCrossing for input instead of DarkSky"),
        )
        .get_matches();
    let electric_file = matches.value_of("electric_file").unwrap();
    let gas_file = matches.value_of("gas_file").unwrap();
    let use_visual_crossing = matches.is_present("visual_crossing");
    let smoothing_days = matches
        .value_of("smoothing_days")
        .unwrap()
        .parse::<u8>()
        .unwrap();

    let client: Box<dyn WeatherClient> = if use_visual_crossing {
        Box::new(VisualCrossingClient::new(
            "4 Bertha Circle,Billerica,MA,USA".to_string(),
            "XHW8QT2FGJKNG25B3RRKPYKKJ".to_string(),
            "visual_crossing_cache".to_string(),
        ))
    } else {
        Box::new(DarkSkyClient::new(
            "9fff3709265bf41d21854d403ed7ee98".to_string(),
            "darksky_cache".to_string(),
        ))
    };
    let mut mgr = TempDataManager::new(client);

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
                (measurements.data.last().unwrap().date - measurements.data[0].date).whole_days()
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
                (measurements.data.last().unwrap().date - measurements.data[0].date).whole_days()
            );

            measurements
        })
    );

    timed!(
        "Drawing graph with smoothing days {}",
        smoothing_days,
        (|| graph_all(electric, gas, &mut mgr, smoothing_days))
    );
}
