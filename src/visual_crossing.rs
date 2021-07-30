use crate::weatherclient::{Temp, WeatherClient};

use flate2::write::{GzDecoder, GzEncoder};
use flate2::Compression;
use reqwest::blocking::{Client, ClientBuilder};
use reqwest::StatusCode;
use rmp_serde::{Deserializer, Serializer};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use time::macros::{date, format_description};
use time::{Date, OffsetDateTime};

use std::cmp::Ordering;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

pub struct VisualCrossingClient {
    my_location: String,
    api_key: String,
    client: Client,
    cache_db: Connection,
}

const TABLE_NAME: &str = "visual_crossing_data";

impl VisualCrossingClient {
    pub fn new(my_location: String, api_key: String, cache_dir: String) -> VisualCrossingClient {
        let mut db_path = PathBuf::from(&cache_dir);
        db_path.push("db");
        db_path.set_extension("sqlite");
        let db_path = db_path.as_path();
        let cache_db = Connection::open(db_path)
            .unwrap_or_else(|err| panic!("Unable to open database {:?}: {}", db_path, err));
        VisualCrossingClient::init_db(&cache_db);

        VisualCrossingClient {
            my_location,
            api_key,
            client: ClientBuilder::new()
                .gzip(true)
                .build()
                .expect("Unable to construct HTTP client"),
            cache_db,
        }
    }

    /// Initialize the DB used to cache NwsResponse objects
    fn init_db(cache_db: &Connection) {
        cache_db
            .execute(
                &format!(
                    "CREATE TABLE IF NOT EXISTS {} (
                        date INTEGER NOT NULL PRIMARY KEY,
                        response BLOB NOT NULL
                    )",
                    TABLE_NAME
                ),
                [],
            )
            .unwrap_or_else(|err| panic!("Unable to create table: {}", err));
    }

    /// Get the VisualCrossing historical data for a date straight from the API
    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn get_from_api(&mut self, date: &Date) -> VisualCrossingResponse {
        let req = self
            .client
            .get(
                "https://weather.visualcrossing.com/VisualCrossingWebServices/rest/services/\
            weatherdata/history",
            )
            .query(&[
                (
                    "startDateTime",
                    format!(
                        "{}T00:00:00",
                        date.format(&format_description!("%Y-%m-%d")).unwrap()
                    ),
                ),
                (
                    "endDateTime",
                    format!(
                        "{}T23:59:59",
                        date.format(&format_description!("%Y-%m-%d")).unwrap()
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
            .unwrap_or_else(|_| panic!("Unable to construct request for date {}", date));
        let url = req.url().clone();
        info!("Calling VisualCrossing: {}", url);
        let res = self
            .client
            .execute(req)
            .expect("Encountered error calling VisualCrossing API");
        match res.status() {
            StatusCode::OK => {
                let obj: VisualCrossingResponse =
                    res.json().expect("Unable to deserialize response");
                obj
            }
            s => panic!("VisualCrossing API returned status {} for URL {}", s, url),
        }
    }

    /// Get the DB key for a given date
    fn get_key(date: &Date) -> i64 {
        let epoch = date!(1970 - 01 - 01);
        (*date - epoch).whole_days()
    }

    /// Read a NwsResponse from the database
    fn read_data(&self, date: &Date) -> Option<VisualCrossingResponse> {
        self.cache_db
            .prepare(&format!(
                "SELECT response FROM {} WHERE date = ?1",
                TABLE_NAME
            ))
            .unwrap_or_else(|err| panic!("Unable to determine if date {} for in DB: {}", date, err))
            .query_map(params![VisualCrossingClient::get_key(date)], |row| {
                Ok(row.get(0).unwrap_or_else(|err| {
                    panic!("Unable to read data from DB row for date {}: {}", date, err)
                }))
            })
            .unwrap_or_else(|err| panic!("Unable to determine if date {} for in DB: {}", date, err))
            .next()
            .map(|x| {
                let response: Vec<u8> = x
                    .unwrap_or_else(|err| panic!("Unable to read data for date {}: {}", date, err));
                VisualCrossingClient::read_blob(response)
            })
    }

    /// Write a VisualCrossingResponse to the database
    fn write_data(&self, date: &Date, response: &VisualCrossingResponse) {
        let encoded = VisualCrossingClient::write_blob(response);
        self.cache_db
            .execute(
                &format!("INSERT INTO {}(date, response) VALUES (?1, ?2)", TABLE_NAME),
                params![VisualCrossingClient::get_key(date), encoded],
            )
            .unwrap_or_else(|err| {
                panic!(
                    "Unable to write NWS data into cache for date {}: {}",
                    date, err
                )
            });
    }

    /// Read a NwsResponse from a MessagePack binary blob
    fn read_blob(raw: Vec<u8>) -> VisualCrossingResponse {
        // decompress
        let mut decompressed = Vec::new();
        let mut decoder = GzDecoder::new(decompressed);
        decoder
            .write_all(&raw[..])
            .unwrap_or_else(|err| panic!("Unable to decompress data: {}", err));
        decompressed = decoder
            .finish()
            .unwrap_or_else(|err| panic!("Unable to decompress data: {}", err));

        // deserialize to object
        let mut de = Deserializer::new(&decompressed[..]);
        let response: VisualCrossingResponse = Deserialize::deserialize(&mut de)
            .unwrap_or_else(|err| panic!("Unable to deserialize data: {}", err));

        response
    }

    /// Write a response to a MessagePack binary blob
    fn write_blob(response: &VisualCrossingResponse) -> Vec<u8> {
        // serialize to buffer
        let mut obj_buf = Vec::new();
        response
            .serialize(&mut Serializer::new(&mut obj_buf))
            .unwrap_or_else(|err| panic!("Unable to serialize data: {}", err));

        // compress buffer
        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        encoder
            .write_all(&obj_buf)
            .unwrap_or_else(|err| panic!("Unable to compress data: {}", err));
        encoder
            .finish()
            .unwrap_or_else(|err| panic!("Unable to compress data: {}", err))
    }
}

impl WeatherClient for VisualCrossingClient {
    fn get_history(&mut self, date: &Date) -> Option<Temp> {
        let date_delta = (*date - OffsetDateTime::now_utc().date()).whole_days();
        let data = match date_delta.cmp(&0) {
            Ordering::Equal => panic!("Cannot get history for today"),
            Ordering::Greater => panic!("Cannot get history for the future"),
            Ordering::Less => {
                let response = self.read_data(date);

                if let Some(resp) = response {
                    resp
                } else {
                    let response = self.get_from_api(date);
                    self.write_data(date, &response);
                    response
                }
            }
        };

        data.locations
            .get(&self.my_location)
            .map(|location| {
                if location.values.len() > 1 {
                    panic!("Found more than one datapoint for day {}", date);
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
                    date.format(&format_description!("%Y-%m-%d")).unwrap()
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
