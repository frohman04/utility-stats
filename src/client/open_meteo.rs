use crate::client::cache::{ClientCache, ClientCacheConnection};
use crate::client::{Temp, WeatherClient};
use reqwest::StatusCode;
use reqwest::blocking::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use time::macros::format_description;
use time::{Date, OffsetDateTime};

pub struct OpenMeteoClient {
    lat: f32,
    lon: f32,
    http_client: Client,
    cache_db: ClientCacheConnection,
}

const TABLE_NAME: &str = "open_meteo";

impl OpenMeteoClient {
    pub fn new(lat: f32, lon: f32, cache: &ClientCache) -> OpenMeteoClient {
        let cache_db = cache.get_connection(TABLE_NAME);
        cache_db.init_db();

        OpenMeteoClient {
            lat,
            lon,
            http_client: ClientBuilder::new()
                .gzip(true)
                .build()
                .expect("Unable to construct HTTP client"),
            cache_db,
        }
    }

    /// Get the VisualCrossing historical data for a date straight from the API
    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn get_from_api(&mut self, date: &Date) -> OpenMeteoResponse {
        let req = self
            .http_client
            .get("https://archive-api.open-meteo.com/v1/archive")
            .query(&[
                (
                    "start_date",
                    date.format(&format_description!("[year]-[month]-[day]"))
                        .unwrap(),
                ),
                (
                    "end_date",
                    date.format(&format_description!("[year]-[month]-[day]"))
                        .unwrap(),
                ),
                ("latitude", self.lat.to_string()),
                ("longitude", self.lon.to_string()),
                (
                    "daily",
                    vec![
                        "temperature_2m_mean",
                        "temperature_2m_max",
                        "temperature_2m_min",
                        "weather_code",
                        "apparent_temperature_mean",
                        "apparent_temperature_max",
                        "apparent_temperature_min",
                        "sunrise",
                        "sunset",
                        "daylight_duration",
                        "sunshine_duration",
                        "precipitation_sum",
                        "rain_sum",
                        "snowfall_sum",
                        "precipitation_hours",
                        "wind_speed_10m_max",
                        "wind_gusts_10m_max",
                        "wind_direction_10m_dominant",
                        "relative_humidity_2m_mean",
                        "relative_humidity_2m_max",
                        "relative_humidity_2m_min",
                        "visibility_mean",
                        "visibility_min",
                        "visibility_max",
                        "winddirection_10m_dominant",
                        "wind_speed_10m_mean",
                        "wind_speed_10m_min",
                        "wet_bulb_temperature_2m_mean",
                        "wet_bulb_temperature_2m_max",
                        "wet_bulb_temperature_2m_min",
                        "pressure_msl_mean",
                        "pressure_msl_max",
                        "pressure_msl_min",
                    ]
                    .join(","),
                ),
                ("timezone", "America/New_York".to_string()),
                ("temperature_unit", "fahrenheit".to_string()),
                ("wind_speed_unit", "mph".to_string()),
                ("precipitation_unit", "inch".to_string()),
            ])
            .build()
            .unwrap_or_else(|_| panic!("Unable to construct request for date {date}"));
        let url = req.url().clone();
        info!("Calling OpenMeteo: {url}");
        let res = self
            .http_client
            .execute(req)
            .expect("Encountered error calling OpenMeteo API");
        match res.status() {
            StatusCode::OK => {
                let obj: OpenMeteoResponse = res.json().expect("Unable to deserialize response");
                obj
            }
            s => panic!("VisualCrossing API returned status {s} for URL {url}"),
        }
    }
}

impl WeatherClient for OpenMeteoClient {
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

        Some(Temp {
            min: data.daily.temperature_2m_min[0],
            max: data.daily.temperature_2m_max[0],
            mean: data.daily.temperature_2m_mean[0],
        })
    }
}

/// API responses consist of a UTF-8-encoded, JSON-formatted object.
#[derive(Debug, Serialize, Deserialize)]
struct OpenMeteoResponse {
    latitude: f32,
    longitude: f32,
    generationtime_ms: f32,
    utc_offset_seconds: i32,
    timezone: String,
    timezone_abbreviation: String,
    elevation: f32,
    daily_units: DailyUnits,
    daily: Daily,
}

#[derive(Debug, Serialize, Deserialize)]
struct DailyUnits {
    time: String,
    temperature_2m_mean: String,
    temperature_2m_max: String,
    temperature_2m_min: String,
    weather_code: String,
    apparent_temperature_mean: String,
    apparent_temperature_max: String,
    apparent_temperature_min: String,
    sunrise: String,
    sunset: String,
    daylight_duration: String,
    sunshine_duration: String,
    precipitation_sum: String,
    rain_sum: String,
    snowfall_sum: String,
    precipitation_hours: String,
    wind_speed_10m_max: String,
    wind_gusts_10m_max: String,
    wind_direction_10m_dominant: String,
    relative_humidity_2m_mean: String,
    relative_humidity_2m_max: String,
    relative_humidity_2m_min: String,
    visibility_mean: String,
    visibility_min: String,
    visibility_max: String,
    winddirection_10m_dominant: String,
    wind_speed_10m_mean: String,
    wind_speed_10m_min: String,
    wet_bulb_temperature_2m_mean: String,
    wet_bulb_temperature_2m_max: String,
    wet_bulb_temperature_2m_min: String,
    pressure_msl_mean: String,
    pressure_msl_max: String,
    pressure_msl_min: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Daily {
    time: Vec<String>,
    temperature_2m_mean: Vec<f32>,
    temperature_2m_max: Vec<f32>,
    temperature_2m_min: Vec<f32>,
    weather_code: Vec<u16>,
    apparent_temperature_mean: Vec<f32>,
    apparent_temperature_max: Vec<f32>,
    apparent_temperature_min: Vec<f32>,
    sunrise: Vec<String>,
    sunset: Vec<String>,
    daylight_duration: Vec<f32>,
    sunshine_duration: Vec<f32>,
    precipitation_sum: Vec<f32>,
    rain_sum: Vec<f32>,
    snowfall_sum: Vec<f32>,
    precipitation_hours: Vec<f32>,
    wind_speed_10m_max: Vec<f32>,
    wind_gusts_10m_max: Vec<f32>,
    wind_direction_10m_dominant: Vec<u16>,
    relative_humidity_2m_mean: Vec<u8>,
    relative_humidity_2m_max: Vec<u8>,
    relative_humidity_2m_min: Vec<u8>,
    visibility_mean: Vec<Option<f32>>,
    visibility_min: Vec<Option<f32>>,
    visibility_max: Vec<Option<f32>>,
    winddirection_10m_dominant: Vec<u16>,
    wind_speed_10m_mean: Vec<f32>,
    wind_speed_10m_min: Vec<f32>,
    wet_bulb_temperature_2m_mean: Vec<f32>,
    wet_bulb_temperature_2m_max: Vec<f32>,
    wet_bulb_temperature_2m_min: Vec<f32>,
    pressure_msl_mean: Vec<f32>,
    pressure_msl_max: Vec<f32>,
    pressure_msl_min: Vec<f32>,
}
