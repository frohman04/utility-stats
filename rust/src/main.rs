extern crate clap;
#[macro_use]
extern crate log;
extern crate simplelog;

#[macro_use]
mod timed;

use clap::{App, Arg};
use simplelog::{CombinedLogger, Config, LevelFilter, TermLogger};

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
        .parse::<i32>()
        .unwrap();

    info!("Reading electric data from {}", electric_file);
    info!("Reading gas data from {}", gas_file);
    info!("Drawing graph with smoothing days {}", smoothing_days);
}
