use crate::client::{Temp, WeatherClient};

use reqwest::StatusCode;
use reqwest::blocking::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use time::macros::format_description;
use time::{Date, OffsetDateTime};

use crate::client::cache::{ClientCache, ClientCacheConnection};
use std::cmp::Ordering;
use std::collections::HashMap;

pub struct VisualCrossingClient {
    my_location: String,
    api_key: String,
    http_client: Client,
    cache_db: ClientCacheConnection,
}

const TABLE_NAME: &str = "visual_crossing";

impl VisualCrossingClient {
    pub fn new(my_location: String, api_key: String, cache: &ClientCache) -> VisualCrossingClient {
        let cache_db = cache.get_connection(TABLE_NAME);
        cache_db.init_db();

        VisualCrossingClient {
            my_location,
            api_key,
            http_client: ClientBuilder::new()
                .gzip(true)
                .build()
                .expect("Unable to construct HTTP client"),
            cache_db,
        }
    }

    /// Get the VisualCrossing historical data for a date straight from the API
    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn get_from_api(&mut self, date: &Date) -> VisualCrossingResponse {
        let req = self
            .http_client
            .get(
                "https://weather.visualcrossing.com/VisualCrossingWebServices/rest/services/\
            weatherdata/history",
            )
            .query(&[
                (
                    "startDateTime",
                    format!(
                        "{}T00:00:00",
                        date.format(&format_description!("[year]-[month]-[day]"))
                            .unwrap()
                    ),
                ),
                (
                    "endDateTime",
                    format!(
                        "{}T23:59:59",
                        date.format(&format_description!("[year]-[month]-[day]"))
                            .unwrap()
                    ),
                ),
                ("location", self.my_location.clone()),
                ("key", self.api_key.clone()),
                ("aggregateHours", "24".to_string()),
                ("collectStationContributions", "true".to_string()),
                ("extendedStats", "true".to_string()),
                ("unitGroup", "us".to_string()),
                ("contentType", "json".to_string()),
            ])
            .build()
            .unwrap_or_else(|_| panic!("Unable to construct request for date {date}"));
        let url = req.url().clone();
        info!("Calling VisualCrossing: {url}");
        let res = self
            .http_client
            .execute(req)
            .expect("Encountered error calling VisualCrossing API");
        match res.status() {
            StatusCode::OK => {
                let obj: VisualCrossingResponse =
                    res.json().expect("Unable to deserialize response");
                obj
            }
            s => panic!("VisualCrossing API returned status {s} for URL {url}"),
        }
    }
}

impl WeatherClient for VisualCrossingClient {
    fn get_history(&mut self, date: &Date) -> Option<Temp> {
        let date_delta = (*date - OffsetDateTime::now_utc().date()).whole_days();
        let data = match date_delta.cmp(&0) {
            Ordering::Equal => panic!("Cannot get history for today"),
            Ordering::Greater => panic!("Cannot get history for the future"),
            Ordering::Less => {
                let response = self.cache_db.read_data(date);

                if let Some(resp) = response {
                    resp
                } else {
                    let response = self.get_from_api(date);
                    self.cache_db.write_data(date, &response);
                    response
                }
            }
        };

        data.locations
            .get(&self.my_location)
            .map(|location| {
                if location.values.len() > 1 {
                    panic!("Found more than one datapoint for day {date}");
                }
                Temp {
                    min: location.values[0].mint,
                    mean: location.values[0].temp,
                    max: location.values[0].maxt,
                }
            })
            .or_else(|| {
                warn!(
                    "No temperature data present for {}",
                    date.format(&format_description!("[year]-[month]-[day]"))
                        .unwrap()
                );
                None
            })
    }
}

/// API responses consist of a UTF-8-encoded, JSON-formatted object.
#[derive(Debug, Serialize, Deserialize)]
pub struct VisualCrossingResponse {
    columns: HashMap<String, Column>,
    locations: HashMap<String, Location>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Column {
    id: String,
    name: String,
    #[serde(rename = "type")]
    typ: u8,
    unit: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Location {
    id: String,
    address: String,
    name: String,
    index: u32,
    latitude: f32,
    longitude: f32,
    distance: f32,
    time: f32,
    tz: String,
    values: Vec<Value>,
    // included with collectStationContribution=true
    #[serde(rename = "stationContributions")]
    station_contributions: HashMap<String, StationContribution>,
    // currentConditions: Option<???>,
    // alerts: Option<???>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Value {
    cloudcover: f32,
    conditions: String,
    datetime: u64,
    #[serde(rename = "datetimeStr")]
    datetime_str: String,
    dew: f32,
    info: Option<String>,
    maxt: f32,
    mint: f32,
    precip: f32,
    precipcover: f32,
    sealevelpressure: f32,
    snow: Option<f32>,
    snowdepth: Option<f32>,
    solarenergy: Option<f32>,
    solarradiation: Option<f32>,
    temp: f32,
    visibility: f32,
    wdir: f32,
    weathertype: String,
    wgust: Option<f32>,

    // included with extendedStats=false
    // heatindex: Option<f32>,
    // humidity: f32,
    // windchill: Option<f32>,
    // wspd: f32,

    // included with extendedStats=true
    min_heatindex: Option<f32>,
    mean_heatindex: Option<f32>,
    max_heatindex: Option<f32>,
    min_humidity: f32,
    mean_humidity: f32,
    max_humidity: f32,
    min_windchill: Option<f32>,
    mean_windchill: Option<f32>,
    max_windchill: Option<f32>,
    min_wspd: f32,
    mean_wspd: f32,
    max_wspd: f32,
    stationinfo: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StationContribution {
    distance: f32,
    latitude: f32,
    longitude: f32,
    #[serde(rename = "useCount")]
    use_count: u16,
    id: String,
    name: String,
    quality: u16,
    contribution: f32,
}
