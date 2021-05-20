use crate::weatherclient::{Temp, WeatherClient};

use flate2::write::{GzDecoder, GzEncoder};
use flate2::Compression;
use reqwest::blocking::{Client, ClientBuilder};
use reqwest::StatusCode;
use rmp_serde::{Deserializer, Serializer};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use time::{Date, Duration, OffsetDateTime};

use std::cmp::Ordering;
use std::io::Write;
use std::path::PathBuf;

pub struct NwsClient {
    my_lat_lon: (f64, f64),
    stations: Vec<String>,
    client: Client,
    cache_db: Connection,
}

const TABLE_NAME: &str = "nws_data";

impl NwsClient {
    pub fn new(my_lat_lon: (f64, f64), stations: Vec<String>, cache_dir: String) -> NwsClient {
        let mut db_path = PathBuf::from(&cache_dir);
        db_path.push("db");
        db_path.set_extension("sqlite");
        let db_path = db_path.as_path();
        let cache_db = Connection::open(db_path)
            .unwrap_or_else(|err| panic!("Unable to open database {:?}: {}", db_path, err));
        NwsClient::init_db(&cache_db);

        NwsClient {
            my_lat_lon,
            stations,
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
                        date INTEGER NOT NULL,
                        station TEXT NOT NULL,
                        response BLOB NOT NULL,
                        PRIMARY KEY(date, station)
                    )",
                    TABLE_NAME
                ),
                [],
            )
            .unwrap_or_else(|err| panic!("Unable to create table: {}", err));
    }

    /// Get the NWS historical data for a date straight from the API
    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn get_from_api(&mut self, date: &Date, station: &str) -> NwsResponse {
        let url = format!(
            "https://api.weather.gov/stations/{}/observations?start={}T00:00:00Z&end={}T00:00:00Z",
            station,
            (*date - Duration::days(1)).format("%Y-%m-%d"),
            date.format("%Y-%m-%d")
        );
        info!("Calling NWS: {}", url);
        let res = self
            .client
            .get(&url)
            .header("Accept", "application/geo+json")
            .header("User-Agent", "utility-stats:rust:reqwest")
            .send()
            .expect("Encountered error calling NWS API");
        match res.status() {
            StatusCode::OK => {
                let obj: NwsResponse = res.json().expect("Unable to deserialize response");
                obj
            }
            s => panic!("NWS API returned status {} for URL {}", s, url),
        }
    }

    /// Get the DB key for a given date
    fn get_key(date: &Date) -> i64 {
        let epoch = date!(1970 - 01 - 01);
        (*date - epoch).whole_days()
    }

    /// Read a NwsResponse from the database
    fn read_data(&self, date: &Date, station: &str) -> Option<NwsResponse> {
        self.cache_db
            .prepare(&format!(
                "SELECT response FROM {} WHERE date = ?1 AND station = ?2",
                TABLE_NAME
            ))
            .unwrap_or_else(|err| {
                panic!(
                    "Unable to determine if date {} for station {} in DB: {}",
                    date, station, err
                )
            })
            .query_map(params![NwsClient::get_key(date), station], |row| {
                Ok(row.get(0).unwrap_or_else(|err| {
                    panic!("Unable to read data from DB row for date {}: {}", date, err)
                }))
            })
            .unwrap_or_else(|err| {
                panic!(
                    "Unable to determine if date {} for station {} in DB: {}",
                    date, station, err
                )
            })
            .next()
            .map(|x| {
                let response: Vec<u8> = x
                    .unwrap_or_else(|err| panic!("Unable to read data for date {}: {}", date, err));
                NwsClient::read_blob(response)
            })
    }

    /// Write a NwsResponse to the database
    fn write_data(&self, date: &Date, station: &str, response: &NwsResponse) {
        let encoded = NwsClient::write_blob(&response);
        self.cache_db
            .execute(
                &format!(
                    "INSERT INTO {}(date, station, response) VALUES (?1, ?2, ?3)",
                    TABLE_NAME
                ),
                params![NwsClient::get_key(date), station, encoded],
            )
            .unwrap_or_else(|err| {
                panic!(
                    "Unable to write NWS data into cache for date {}, station {}: {}",
                    date, station, err
                )
            });
    }

    /// Read a NwsResponse from a MessagePack binary blob
    fn read_blob(raw: Vec<u8>) -> NwsResponse {
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
        let response: NwsResponse = Deserialize::deserialize(&mut de)
            .unwrap_or_else(|err| panic!("Unable to deserialize data: {}", err));

        response
    }

    /// Write a response to a MessagePack binary blob
    fn write_blob(response: &NwsResponse) -> Vec<u8> {
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

impl WeatherClient for NwsClient {
    fn get_history(&mut self, date: &Date) -> Option<Temp> {
        let date_delta = (*date - OffsetDateTime::now_utc().date()).whole_days();
        let station = self.stations.first().unwrap().clone();
        let data = match date_delta.cmp(&0) {
            Ordering::Equal => panic!("Cannot get history for today"),
            Ordering::Greater => panic!("Cannot get history for the future"),
            Ordering::Less => {
                let response = self.read_data(date, &station);

                if let Some(resp) = response {
                    resp
                } else {
                    let response = self.get_from_api(date, &station);
                    self.write_data(date, &station, &response);
                    response
                }
            }
        };

        let temps: Vec<f32> = data
            .features
            .into_iter()
            .map(|f| {
                if f.properties.temperature.unit_code == "unit:degC" {
                    (f.properties.temperature.value * 9f32 / 5f32) + 32f32
                } else {
                    panic!(
                        "Unknown temperature unit code: {}",
                        f.properties.temperature.unit_code
                    )
                }
            })
            .collect();
        if !temps.is_empty() {
            let mut min = f32::MAX;
            let mut max = f32::MIN;
            let mut sum = 0 as f32;
            let mut count = 0;

            for temp in temps {
                if temp < min {
                    min = temp;
                }
                if temp > max {
                    max = temp;
                }
                sum += temp;
                count += 1;
            }

            if count > 0 {
                Some(Temp {
                    min,
                    mean: sum / count as f32,
                    max,
                })
            } else {
                None
            }
        } else {
            warn!(
                "No temperature data present for {}",
                date.format("%Y-%m-%d")
            );
            None
        }
    }
}

/// API responses consist of a UTF-8-encoded, JSON-formatted object.
#[derive(Debug, Serialize, Deserialize)]
pub struct NwsResponse {
    pub features: Vec<NwsFeature>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NwsFeature {
    pub id: String,
    #[serde(rename = "type")]
    pub typ: String,
    pub geometry: NwsGeometryPoint,
    pub properties: NwsProperties,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NwsGeometryPoint {
    #[serde(rename = "type")]
    pub typ: String,
    pub coordinates: [f64; 2],
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NwsProperties {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@type")]
    pub typ: String,
    pub elevation: NwsElevation,
    pub station: String,
    pub timestamp: OffsetDateTime,
    #[serde(rename = "rawMessage")]
    pub raw_message: String,
    #[serde(rename = "textDescription")]
    pub text_description: String,
    pub icon: String,
    // #[serde(rename="presentWeather")]
    // pub present_weather: Vec<???>,
    pub temperature: NwsMeasurement,
    pub dewpoint: NwsMeasurement,
    #[serde(rename = "windDirection")]
    pub wind_direction: NwsMeasurement,
    #[serde(rename = "windSpeed")]
    pub wind_speed: NwsMeasurement,
    #[serde(rename = "windGust")]
    pub wind_gust: NwsMeasurement,
    #[serde(rename = "barometricPressure")]
    pub barometric_pressure: NwsMeasurement,
    #[serde(rename = "sealevelPressure")]
    pub sealevel_pressure: NwsMeasurement,
    pub visibility: NwsMeasurement,
    #[serde(rename = "maxTemperatureLast24Hours")]
    pub max_temp_last_24_hours: NwsMeasurement,
    #[serde(rename = "minTemperatureLast24Hours")]
    pub min_temp_last_24_hours: NwsMeasurement,
    #[serde(rename = "precipitationLastHour")]
    pub precip_last_1_hour: NwsMeasurement,
    #[serde(rename = "precipitationLast3Hours")]
    pub precip_last_3_hours: NwsMeasurement,
    #[serde(rename = "precipitationLast6Hours")]
    pub precip_last_6_hours: NwsMeasurement,
    #[serde(rename = "relativeHumidity")]
    pub relative_humidity: NwsMeasurement,
    #[serde(rename = "windChill")]
    pub wind_chill: NwsMeasurement,
    #[serde(rename = "heatIndex")]
    pub heat_index: NwsMeasurement,
    #[serde(rename = "cloudLayers")]
    pub cloud_layers: Vec<NwsCloudLayer>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NwsElevation {
    pub value: i32,
    #[serde(rename = "unitCode")]
    pub unit_code: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NwsMeasurement {
    pub value: f32,
    #[serde(rename = "unitCode")]
    pub unit_code: String,
    #[serde(rename = "qualityControl")]
    pub quality_control: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NwsCloudLayer {
    pub base: NwsCloudLayerBase,
    pub amount: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NwsCloudLayerBase {
    pub value: u32,
    #[serde(rename = "unitCode")]
    pub unit_code: String,
}
