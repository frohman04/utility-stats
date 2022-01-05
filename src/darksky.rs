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
use std::io::Write;
use std::path::PathBuf;

pub struct DarkSkyClient {
    api_key: String,
    client: Client,
    request_count: u32,
    cache_db: Connection,
}

const TABLE_NAME: &str = "darksky_data";

impl DarkSkyClient {
    /// Construct a new client that uses the given API key
    pub fn new(api_key: String, cache_dir: String) -> DarkSkyClient {
        let mut db_path = PathBuf::from(&cache_dir);
        db_path.push("db");
        db_path.set_extension("sqlite");
        let db_path = db_path.as_path();
        let cache_db = Connection::open(db_path)
            .unwrap_or_else(|err| panic!("Unable to open database {:?}: {}", db_path, err));
        DarkSkyClient::init_db(&cache_db);

        DarkSkyClient {
            api_key,
            client: ClientBuilder::new()
                .gzip(true)
                .build()
                .expect("Unable to construct HTTP client"),
            request_count: 0,
            cache_db,
        }
    }

    /// Get the DarkSky historical data for a date straight from the API
    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn get_from_api(&mut self, date: &Date) -> DarkSkyResponse {
        self.request_count += 1;
        if self.request_count >= 1000 {
            panic!("Can only make 1000 requests per day");
        }

        let url = format!(
            "https://api.darksky.net/forecast/{}/42.5468,-71.2550102,{}T00:00:00",
            self.api_key,
            date.format(&format_description!("[year]-[month]-[day]"))
                .unwrap()
        );
        info!("Calling DarkSky: {}", url);
        let res = self
            .client
            .get(&url)
            .send()
            .expect("Encountered error calling DarkSky API");
        match res.status() {
            StatusCode::OK => {
                let obj: DarkSkyResponse = res.json().expect("Unable to deserialize response");
                obj
            }
            s => panic!("DarkSky API returned status {} for URL {}", s, url),
        }
    }

    /// Initialize the DB used to cache DarkSkyResponse objects
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

    /// Get the DB key for a given date
    fn get_key(date: &Date) -> i64 {
        let epoch = date!(1970 - 01 - 01);
        (*date - epoch).whole_days()
    }

    /// Read a DarkSkyResponse from the database
    fn read_data(&self, date: &Date) -> Option<DarkSkyResponse> {
        self.cache_db
            .prepare(&format!(
                "SELECT response FROM {} WHERE date = ?1",
                TABLE_NAME
            ))
            .unwrap_or_else(|err| panic!("Unable to determine if date {} in DB: {}", date, err))
            .query_map(params![DarkSkyClient::get_key(date)], |row| {
                Ok(row.get(0).unwrap_or_else(|err| {
                    panic!("Unable to read data from DB row for date {}: {}", date, err)
                }))
            })
            .unwrap_or_else(|err| panic!("Unable to determine if date {} in DB: {}", date, err))
            .next()
            .map(|x| {
                let response: Vec<u8> = x
                    .unwrap_or_else(|err| panic!("Unable to read data for date {}: {}", date, err));
                DarkSkyClient::read_blob(response)
            })
    }

    /// Write a DarkSkyResponse to the database
    fn write_data(&self, date: &Date, response: &DarkSkyResponse) {
        let encoded = DarkSkyClient::write_blob(response);
        self.cache_db
            .execute(
                &format!("INSERT INTO {}(date, response) VALUES (?1, ?2)", TABLE_NAME),
                params![DarkSkyClient::get_key(date), encoded],
            )
            .unwrap_or_else(|err| {
                panic!(
                    "Unable to write DarkSky data into cache for date {}: {}",
                    date, err
                )
            });
    }

    /// Read a DarkSkyResponse from a MessagePack binary blob
    fn read_blob(raw: Vec<u8>) -> DarkSkyResponse {
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
        let response: DarkSkyResponse = Deserialize::deserialize(&mut de)
            .unwrap_or_else(|err| panic!("Unable to deserialize data: {}", err));

        response
    }

    /// Write a response to a MessagePack binary blob
    fn write_blob(response: &DarkSkyResponse) -> Vec<u8> {
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

impl WeatherClient for DarkSkyClient {
    /// Get the temperature history for a given day from DarkSky
    #[allow(clippy::trivially_copy_pass_by_ref)]
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

        if data.hourly.is_some() {
            let temps: Vec<f32> = data
                .hourly
                .unwrap()
                .data
                .into_iter()
                .filter_map(|dp| dp.temperature)
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
                warn!("No temperature data present for {:?}", date);
                None
            }
        } else {
            None
        }
    }
}

/// API responses consist of a UTF-8-encoded, JSON-formatted object.
#[derive(Debug, Serialize, Deserialize)]
pub struct DarkSkyResponse {
    /// The requested latitude.
    pub latitude: f32,
    /// The requested longitude.
    pub longitude: f32,
    /// The IANA timezone name for the requested location. This is used for text summaries and for
    /// determining when hourly and daily data block objects begin.
    pub timezone: String,
    /// deprecated.  The current timezone offset in hours. (Use of this property will almost
    /// certainly result in Daylight Saving Time bugs. Please use timezone, instead.)
    pub offset: i8,
    /// A data point containing the current weather conditions at the requested location.
    pub currently: Option<DataPointCurrently>,
    /// A data block containing the weather conditions minute-by-minute for the next hour.
    pub minutely: Option<DataBlock<DataPointMinutely>>,
    /// A data block containing the weather conditions hour-by-hour for the next two days.
    pub hourly: Option<DataBlock<DataPointHourly>>,
    /// A data block containing the weather conditions day-by-day for the next week.
    pub daily: Option<DataBlock<DataPointDaily>>,
    /// An alerts array, which, if present, contains any severe weather alerts pertinent to the
    /// requested location.
    pub alerts: Option<Vec<Alert>>,
    /// A flags object containing miscellaneous metadata about the request.
    pub flags: Option<Flags>,
}

/// A data block object represents the various weather phenomena occurring over a period of time.
#[derive(Debug, Serialize, Deserialize)]
pub struct DataBlock<DP> {
    /// An array of data points, ordered by time, which together describe the weather conditions at
    /// the requested location over time.
    pub data: Vec<DP>,
    /// A human-readable summary of this data block.
    pub summary: Option<String>,
    /// A machine-readable text summary of this data block. (May take on the same values as the icon
    /// property of data points.)
    pub icon: Option<String>,
}

/// A data point object contains various properties, each representing the average (unless otherwise
/// specified) of a particular weather phenomenon occurring during a period of time: an instant in
/// the case of currently, a minute for minutely, an hour for hourly, and a day for daily
#[derive(Debug, Serialize, Deserialize)]
pub struct DataPointCurrently {
    /// The UNIX time at which this data point begins. minutely data point are always aligned to the
    /// top of the minute, hourly data point objects to the top of the hour, and daily data point
    /// objects to midnight of the day, all according to the local time zone.
    #[serde(alias = "time")]
    pub timestamp: i64,
    /// A human-readable text summary of this data point. (This property has millions of possible
    /// values, so don’t use it for automated purposes: use the icon property, instead!)
    pub summary: Option<String>,
    /// A machine-readable text summary of this data point, suitable for selecting an icon for
    /// display. If defined, this property will have one of the following values: clear-day,
    /// clear-night, rain, snow, sleet, wind, fog, cloudy, partly-cloudy-day, or
    /// partly-cloudy-night. (Developers should ensure that a sensible default is defined, as
    /// additional values, such as hail, thunderstorm, or tornado, may be defined in the future.)
    pub icon: Option<String>,
    /// The intensity (in inches of liquid water per hour) of precipitation occurring at the given
    /// time. This value is conditional on probability (that is, assuming any precipitation occurs
    /// at all).
    #[serde(alias = "precipIntensity")]
    pub precip_intensity: Option<f32>,
    /// The standard deviation of the distribution of precipIntensity. (We only return this property
    /// when the full distribution, and not merely the expected mean, can be estimated with
    /// accuracy.)
    #[serde(alias = "precipIntensityError")]
    pub precip_intensity_error: Option<f32>,
    /// The probability of precipitation occurring, between 0 and 1, inclusive.
    #[serde(alias = "precipProbability")]
    pub precip_probability: Option<f32>,
    /// The dew point in degrees Fahrenheit.
    #[serde(alias = "dewPoint")]
    pub dew_point: Option<f32>,
    /// The relative humidity, between 0 and 1, inclusive.
    pub humidity: Option<f32>,
    /// The sea-level air pressure in millibars.
    pub pressure: Option<f32>,
    /// The wind speed in miles per hour.
    #[serde(alias = "windSpeed")]
    pub wind_speed: Option<f32>,
    /// The wind gust speed in miles per hour.
    #[serde(alias = "windGust")]
    pub wind_gust: Option<f32>,
    /// The time at which the maximum wind gust speed occurs during the day.
    #[serde(alias = "windGustTime")]
    pub wind_gust_time: Option<u64>,
    /// The direction that the wind is coming from in degrees, with true north at 0° and progressing
    /// clockwise. (If windSpeed is zero, then this value will not be defined.)
    #[serde(alias = "windBearing")]
    pub wind_bearing: Option<i16>,
    /// The percentage of sky occluded by clouds, between 0 and 1, inclusive.
    #[serde(alias = "cloudCover")]
    pub cloud_cover: Option<f32>,
    /// The UV index.
    #[serde(alias = "uvIndex")]
    pub uv_index: Option<u8>,
    /// The average visibility in miles, capped at 10 miles.
    pub visibility: Option<f32>,
    /// The approximate direction of the nearest storm in degrees, with true north at 0° and
    /// progressing clockwise. (If nearestStormDistance is zero, then this value will not be
    /// defined.) (only on currently)
    #[serde(alias = "nearestStormBearing")]
    pub nearest_storm_bearing: Option<i16>,
    /// The approximate distance to the nearest storm in miles. (A storm distance of 0 doesn’t
    /// necessarily refer to a storm at the requested location, but rather a storm in the vicinity
    /// of that location.) (only on currently)
    #[serde(alias = "nearestStormDistance")]
    pub nearest_storm_distance: Option<f32>,
    /// The columnar density of total atmospheric ozone at the given time in Dobson units.
    pub ozone: Option<f32>,
}

impl DataPointCurrently {
    /// The UNIX time at which this data point begins. minutely data point are always aligned to the
    /// top of the minute, hourly data point objects to the top of the hour, and daily data point
    /// objects to midnight of the day, all according to the local time zone.
    #[allow(dead_code)]
    pub fn time(&self) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(self.timestamp).unwrap()
    }
}

/// A data point object contains various properties, each representing the average (unless otherwise
/// specified) of a particular weather phenomenon occurring during a period of time: an instant in
/// the case of currently, a minute for minutely, an hour for hourly, and a day for daily
#[derive(Debug, Serialize, Deserialize)]
pub struct DataPointMinutely {
    /// The UNIX time at which this data point begins. minutely data point are always aligned to the
    /// top of the minute, hourly data point objects to the top of the hour, and daily data point
    /// objects to midnight of the day, all according to the local time zone.
    #[serde(alias = "time")]
    pub timestamp: i64,
    /// A human-readable text summary of this data point. (This property has millions of possible
    /// values, so don’t use it for automated purposes: use the icon property, instead!)
    pub summary: Option<String>,
    /// A machine-readable text summary of this data point, suitable for selecting an icon for
    /// display. If defined, this property will have one of the following values: clear-day,
    /// clear-night, rain, snow, sleet, wind, fog, cloudy, partly-cloudy-day, or
    /// partly-cloudy-night. (Developers should ensure that a sensible default is defined, as
    /// additional values, such as hail, thunderstorm, or tornado, may be defined in the future.)
    pub icon: Option<String>,
    /// The intensity (in inches of liquid water per hour) of precipitation occurring at the given
    /// time. This value is conditional on probability (that is, assuming any precipitation occurs
    /// at all).
    #[serde(alias = "precipIntensity")]
    pub precip_intensity: Option<f32>,
    /// The standard deviation of the distribution of precipIntensity. (We only return this property
    /// when the full distribution, and not merely the expected mean, can be estimated with
    /// accuracy.)
    #[serde(alias = "precipIntensityError")]
    pub precip_intensity_error: Option<f32>,
    /// The probability of precipitation occurring, between 0 and 1, inclusive.
    #[serde(alias = "precipProbability")]
    pub precip_probability: Option<f32>,
    /// The type of precipitation occurring at the given time. If defined, this property will have
    /// one of the following values: "rain", "snow", or "sleet" (which refers to each of freezing
    /// rain, ice pellets, and “wintery mix”). (If precipIntensity is zero, then this property will
    /// not be defined. Additionally, due to the lack of data in our sources, historical precipType
    /// information is usually estimated, rather than observed.)
    #[serde(alias = "precipType")]
    pub precip_type: Option<String>,
    /// The dew point in degrees Fahrenheit.
    #[serde(alias = "dewPoint")]
    pub dew_point: Option<f32>,
    /// The relative humidity, between 0 and 1, inclusive.
    pub humidity: Option<f32>,
    /// The sea-level air pressure in millibars.
    pub pressure: Option<f32>,
    /// The wind speed in miles per hour.
    #[serde(alias = "windSpeed")]
    pub wind_speed: Option<f32>,
    /// The wind gust speed in miles per hour.
    #[serde(alias = "windGust")]
    pub wind_gust: Option<f32>,
    /// The time at which the maximum wind gust speed occurs during the day.
    #[serde(alias = "windGustTime")]
    pub wind_gust_time: Option<u64>,
    /// The direction that the wind is coming from in degrees, with true north at 0° and progressing
    /// clockwise. (If windSpeed is zero, then this value will not be defined.)
    #[serde(alias = "windBearing")]
    pub wind_bearing: Option<i16>,
    /// The percentage of sky occluded by clouds, between 0 and 1, inclusive.
    #[serde(alias = "cloudCover")]
    pub cloud_cover: Option<f32>,
    /// The UV index.
    #[serde(alias = "uvIndex")]
    pub uv_index: Option<u8>,
    /// The average visibility in miles, capped at 10 miles.
    pub visibility: Option<f32>,
    /// The columnar density of total atmospheric ozone at the given time in Dobson units.
    pub ozone: Option<f32>,
}

impl DataPointMinutely {
    /// The UNIX time at which this data point begins. minutely data point are always aligned to the
    /// top of the minute, hourly data point objects to the top of the hour, and daily data point
    /// objects to midnight of the day, all according to the local time zone.
    #[allow(dead_code)]
    pub fn time(&self) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(self.timestamp).unwrap()
    }
}

/// A data point object contains various properties, each representing the average (unless otherwise
/// specified) of a particular weather phenomenon occurring during a period of time: an instant in
/// the case of currently, a minute for minutely, an hour for hourly, and a day for daily
#[derive(Debug, Serialize, Deserialize)]
pub struct DataPointHourly {
    /// The UNIX time at which this data point begins. minutely data point are always aligned to the
    /// top of the minute, hourly data point objects to the top of the hour, and daily data point
    /// objects to midnight of the day, all according to the local time zone.
    #[serde(alias = "time")]
    pub timestamp: i64,
    /// A human-readable text summary of this data point. (This property has millions of possible
    /// values, so don’t use it for automated purposes: use the icon property, instead!)
    pub summary: Option<String>,
    /// A machine-readable text summary of this data point, suitable for selecting an icon for
    /// display. If defined, this property will have one of the following values: clear-day,
    /// clear-night, rain, snow, sleet, wind, fog, cloudy, partly-cloudy-day, or
    /// partly-cloudy-night. (Developers should ensure that a sensible default is defined, as
    /// additional values, such as hail, thunderstorm, or tornado, may be defined in the future.)
    pub icon: Option<String>,
    /// The intensity (in inches of liquid water per hour) of precipitation occurring at the given
    /// time. This value is conditional on probability (that is, assuming any precipitation occurs
    /// at all).
    #[serde(alias = "precipIntensity")]
    pub precip_intensity: Option<f32>,
    /// The standard deviation of the distribution of precipIntensity. (We only return this property
    /// when the full distribution, and not merely the expected mean, can be estimated with
    /// accuracy.)
    #[serde(alias = "precipIntensityError")]
    pub precip_intensity_error: Option<f32>,
    /// The probability of precipitation occurring, between 0 and 1, inclusive.
    #[serde(alias = "precipProbability")]
    pub precip_probability: Option<f32>,
    /// The type of precipitation occurring at the given time. If defined, this property will have
    /// one of the following values: "rain", "snow", or "sleet" (which refers to each of freezing
    /// rain, ice pellets, and “wintery mix”). (If precipIntensity is zero, then this property will
    /// not be defined. Additionally, due to the lack of data in our sources, historical precipType
    /// information is usually estimated, rather than observed.)
    #[serde(alias = "precipType")]
    pub precip_type: Option<String>,
    /// The amount of snowfall accumulation expected to occur, in inches. (If no snowfall is
    /// expected, this property will not be defined.) (only on hourly and daily)
    #[serde(alias = "precipAccumulation")]
    pub precip_accumulation: Option<f32>,
    /// The air temperature in degrees Fahrenheit. (only on hourly)
    pub temperature: Option<f32>,
    /// The apparent (or “feels like”) temperature in degrees Fahrenheit. (only on hourly)
    #[serde(alias = "apparentTemperature")]
    pub apparent_temperature: Option<f32>,
    /// The dew point in degrees Fahrenheit.
    #[serde(alias = "dewPoint")]
    pub dew_point: Option<f32>,
    /// The relative humidity, between 0 and 1, inclusive.
    pub humidity: Option<f32>,
    /// The sea-level air pressure in millibars.
    pub pressure: Option<f32>,
    /// The wind speed in miles per hour.
    #[serde(alias = "windSpeed")]
    pub wind_speed: Option<f32>,
    /// The wind gust speed in miles per hour.
    #[serde(alias = "windGust")]
    pub wind_gust: Option<f32>,
    /// The time at which the maximum wind gust speed occurs during the day.
    #[serde(alias = "windGustTime")]
    pub wind_gust_time: Option<u64>,
    /// The direction that the wind is coming from in degrees, with true north at 0° and progressing
    /// clockwise. (If windSpeed is zero, then this value will not be defined.)
    #[serde(alias = "windBearing")]
    pub wind_bearing: Option<i16>,
    /// The percentage of sky occluded by clouds, between 0 and 1, inclusive.
    #[serde(alias = "cloudCover")]
    pub cloud_cover: Option<f32>,
    /// The UV index.
    #[serde(alias = "uvIndex")]
    pub uv_index: Option<u8>,
    /// The average visibility in miles, capped at 10 miles.
    pub visibility: Option<f32>,
    /// The columnar density of total atmospheric ozone at the given time in Dobson units.
    pub ozone: Option<f32>,
}

impl DataPointHourly {
    /// The UNIX time at which this data point begins. minutely data point are always aligned to the
    /// top of the minute, hourly data point objects to the top of the hour, and daily data point
    /// objects to midnight of the day, all according to the local time zone.
    #[allow(dead_code)]
    pub fn time(&self) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(self.timestamp).unwrap()
    }
}

/// A data point object contains various properties, each representing the average (unless otherwise
/// specified) of a particular weather phenomenon occurring during a period of time: an instant in
/// the case of currently, a minute for minutely, an hour for hourly, and a day for daily
#[derive(Debug, Serialize, Deserialize)]
pub struct DataPointDaily {
    /// The UNIX time at which this data point begins. minutely data point are always aligned to the
    /// top of the minute, hourly data point objects to the top of the hour, and daily data point
    /// objects to midnight of the day, all according to the local time zone.
    #[serde(alias = "time")]
    pub timestamp: i64,
    /// A human-readable text summary of this data point. (This property has millions of possible
    /// values, so don’t use it for automated purposes: use the icon property, instead!)
    pub summary: Option<String>,
    /// A machine-readable text summary of this data point, suitable for selecting an icon for
    /// display. If defined, this property will have one of the following values: clear-day,
    /// clear-night, rain, snow, sleet, wind, fog, cloudy, partly-cloudy-day, or
    /// partly-cloudy-night. (Developers should ensure that a sensible default is defined, as
    /// additional values, such as hail, thunderstorm, or tornado, may be defined in the future.)
    pub icon: Option<String>,
    /// The intensity (in inches of liquid water per hour) of precipitation occurring at the given
    /// time. This value is conditional on probability (that is, assuming any precipitation occurs
    /// at all).
    #[serde(alias = "precipIntensity")]
    pub precip_intensity: Option<f32>,
    /// The standard deviation of the distribution of precipIntensity. (We only return this property
    /// when the full distribution, and not merely the expected mean, can be estimated with
    /// accuracy.)
    #[serde(alias = "precipIntensityError")]
    pub precip_intensity_error: Option<f32>,
    /// The maximum value of precipIntensity during a given day. (only on daily)
    #[serde(alias = "precipIntensityMax")]
    pub precip_intensity_max: Option<f32>,
    /// The UNIX time of when precipIntensityMax occurs during a given day. (only on daily)
    #[serde(alias = "precipIntensityMaxTime")]
    pub precip_intensity_max_timestamp: Option<u64>,
    /// The probability of precipitation occurring, between 0 and 1, inclusive.
    #[serde(alias = "precipProbability")]
    pub precip_probability: Option<f32>,
    /// The type of precipitation occurring at the given time. If defined, this property will have
    /// one of the following values: "rain", "snow", or "sleet" (which refers to each of freezing
    /// rain, ice pellets, and “wintery mix”). (If precipIntensity is zero, then this property will
    /// not be defined. Additionally, due to the lack of data in our sources, historical precipType
    /// information is usually estimated, rather than observed.)
    #[serde(alias = "precipType")]
    pub precip_type: Option<String>,
    /// The amount of snowfall accumulation expected to occur, in inches. (If no snowfall is
    /// expected, this property will not be defined.) (only on hourly and daily)
    #[serde(alias = "precipAccumulation")]
    pub precip_accumulation: Option<f32>,
    /// The daytime high temperature. (only on daily)
    #[serde(alias = "temperatureHigh")]
    pub temperature_high: Option<f32>,
    /// The UNIX time representing when the daytime high temperature occurs. (only on daily)
    #[serde(alias = "temperatureHighTime")]
    pub temperature_high_timestamp: Option<u64>,
    /// The overnight low temperature. (only on daily)
    #[serde(alias = "temperatureLow")]
    pub temperature_low: Option<f32>,
    /// The UNIX time representing when the overnight low temperature occurs. (only on daily)
    #[serde(alias = "temperatureLowTime")]
    pub temperature_low_timestamp: Option<f32>,
    /// The maximum temperature during a given date. (only on daily)
    #[serde(alias = "temperatureMax")]
    pub temperature_max: Option<f32>,
    /// The UNIX time representing when the maximum temperature during a given date occurs. (only
    /// on daily)
    #[serde(alias = "temperatureMaxTime")]
    pub temperature_max_timestamp: Option<u64>,
    /// The minimum temperature during a given date. (only on daily)
    #[serde(alias = "temperatureMin")]
    pub temperature_min: Option<f32>,
    /// The UNIX time representing when the minimum temperature during a given date occurs. (only
    /// on daily)
    #[serde(alias = "temperatureMinTime")]
    pub temperature_min_timestamp: Option<u64>,
    /// The daytime high apparent temperature. (only on daily)
    #[serde(alias = "apparentTemperatureHigh")]
    pub apparent_temperature_high: Option<f32>,
    /// The UNIX time representing when the daytime high apparent temperature occurs.
    /// (only on daily)
    #[serde(alias = "apparentTemperatureHighTime")]
    pub apparent_temperature_high_timestamp: Option<i64>,
    /// The overnight low apparent temperature. (only on daily)
    #[serde(alias = "apparentTemperatureLow")]
    pub apparent_temperature_low: Option<f32>,
    /// The UNIX time representing when the overnight low apparent temperature occurs.
    /// (only on daily)
    #[serde(alias = "apparentTemperatureLowTime")]
    pub apparent_temperature_low_timestamp: Option<u64>,
    /// The maximum apparent temperature during a given date. (only on daily)
    #[serde(alias = "apparentTemperatureMax")]
    pub apparent_temperature_max: Option<f32>,
    /// The UNIX time representing when the maximum apparent temperature during a given date occurs.
    /// (only on daily)
    #[serde(alias = "apparentTemperatureMaxTime")]
    pub apparent_temperature_max_timestamp: Option<f32>,
    /// The minimum apparent temperature during a given date. (only on daily)
    #[serde(alias = "apparentTemperatureMin")]
    pub apparent_temperature_min: Option<f32>,
    /// The UNIX time representing when the minimum apparent temperature during a given date occurs.
    /// (only on daily)
    #[serde(alias = "apparentTemperatureMinTime")]
    pub apparent_temperature_min_timestamp: Option<u64>,
    /// The dew point in degrees Fahrenheit.
    #[serde(alias = "dewPoint")]
    pub dew_point: Option<f32>,
    /// The relative humidity, between 0 and 1, inclusive.
    pub humidity: Option<f32>,
    /// The sea-level air pressure in millibars.
    pub pressure: Option<f32>,
    /// The wind speed in miles per hour.
    #[serde(alias = "windSpeed")]
    pub wind_speed: Option<f32>,
    /// The wind gust speed in miles per hour.
    #[serde(alias = "windGust")]
    pub wind_gust: Option<f32>,
    /// The time at which the maximum wind gust speed occurs during the day.
    #[serde(alias = "windGustTime")]
    pub wind_gust_time: Option<u64>,
    /// The direction that the wind is coming from in degrees, with true north at 0° and progressing
    /// clockwise. (If windSpeed is zero, then this value will not be defined.)
    #[serde(alias = "windBearing")]
    pub wind_bearing: Option<i16>,
    /// The percentage of sky occluded by clouds, between 0 and 1, inclusive.
    #[serde(alias = "cloudCover")]
    pub cloud_cover: Option<f32>,
    /// The UV index.
    #[serde(alias = "uvIndex")]
    pub uv_index: Option<u8>,
    /// The UNIX time of when the maximum uvIndex occurs during a given day. (only on daily)
    #[serde(alias = "uvIndexTime")]
    pub uv_index_timestamp: Option<u64>,
    /// The average visibility in miles, capped at 10 miles.
    pub visibility: Option<f32>,
    /// The fractional part of the lunation number during the given day: a value of 0 corresponds
    /// to a new moon, 0.25 to a first quarter moon, 0.5 to a full moon, and 0.75 to a last quarter
    /// moon. (The ranges in between these represent waxing crescent, waxing gibbous, waning
    /// gibbous, and waning crescent moons, respectively.) (only on daily)
    #[serde(alias = "moonPhase")]
    pub moon_phase: Option<f32>,
    /// The columnar density of total atmospheric ozone at the given time in Dobson units.
    pub ozone: Option<f32>,
    /// The UNIX time of when the sun will rise during a given day. (only on daily)
    #[serde(alias = "sunriseTime")]
    pub sunrise_timestamp: Option<u64>,
    /// The UNIX time of when the sun will set during a given day. (only on daily)
    #[serde(alias = "sunsetTime")]
    pub sunset_timestamp: Option<u64>,
}

impl DataPointDaily {
    /// The UNIX time at which this data point begins. minutely data point are always aligned to the
    /// top of the minute, hourly data point objects to the top of the hour, and daily data point
    /// objects to midnight of the day, all according to the local time zone.
    #[allow(dead_code)]
    pub fn time(&self) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(self.timestamp).unwrap()
    }

    /// The time of when precipIntensityMax occurs during a given day. (only on daily)
    #[allow(dead_code)]
    pub fn precip_intensity_max_time(&self) -> Option<OffsetDateTime> {
        self.precip_intensity_max_timestamp
            .map(|x| OffsetDateTime::from_unix_timestamp(x as i64).unwrap())
    }

    /// The time representing when the daytime high temperature occurs. (only on daily)
    #[allow(dead_code)]
    pub fn temperature_high_time(&self) -> Option<OffsetDateTime> {
        self.temperature_high_timestamp
            .map(|x| OffsetDateTime::from_unix_timestamp(x as i64).unwrap())
    }

    /// The time representing when the overnight low temperature occurs. (only on daily)
    #[allow(dead_code)]
    pub fn temperature_low_time(&self) -> Option<OffsetDateTime> {
        self.temperature_low_timestamp
            .map(|x| OffsetDateTime::from_unix_timestamp(x as i64).unwrap())
    }

    /// The time representing when the maximum temperature during a given date occurs. (only
    /// on daily)
    #[allow(dead_code)]
    pub fn temperature_max_time(&self) -> Option<OffsetDateTime> {
        self.temperature_max_timestamp
            .map(|x| OffsetDateTime::from_unix_timestamp(x as i64).unwrap())
    }

    /// The time representing when the minimum temperature during a given date occurs. (only
    /// on daily)
    #[allow(dead_code)]
    pub fn temperature_min_time(&self) -> Option<OffsetDateTime> {
        self.temperature_min_timestamp
            .map(|x| OffsetDateTime::from_unix_timestamp(x as i64).unwrap())
    }

    /// The time representing when the daytime high apparent temperature occurs. (only on daily)
    #[allow(dead_code)]
    pub fn apparent_temperature_high_time(&self) -> Option<OffsetDateTime> {
        self.apparent_temperature_high_timestamp
            .map(|x| OffsetDateTime::from_unix_timestamp(x as i64).unwrap())
    }

    /// The time representing when the overnight low apparent temperature occurs. (only on daily)
    #[allow(dead_code)]
    pub fn apparent_temperature_low_time(&self) -> Option<OffsetDateTime> {
        self.apparent_temperature_low_timestamp
            .map(|x| OffsetDateTime::from_unix_timestamp(x as i64).unwrap())
    }

    /// The time representing when the maximum apparent temperature during a given date occurs.
    /// (only on daily)
    #[allow(dead_code)]
    pub fn apparent_temperature_max_time(&self) -> Option<OffsetDateTime> {
        self.apparent_temperature_max_timestamp
            .map(|x| OffsetDateTime::from_unix_timestamp(x as i64).unwrap())
    }

    /// The time representing when the minimum apparent temperature during a given date occurs.
    /// (only on daily)
    #[allow(dead_code)]
    pub fn apparent_temperature_min_time(&self) -> Option<OffsetDateTime> {
        self.apparent_temperature_min_timestamp
            .map(|x| OffsetDateTime::from_unix_timestamp(x as i64).unwrap())
    }

    /// The time of when the maximum uvIndex occurs during a given day. (only on daily)
    #[allow(dead_code)]
    pub fn uv_index_time(&self) -> Option<OffsetDateTime> {
        self.uv_index_timestamp
            .map(|x| OffsetDateTime::from_unix_timestamp(x as i64).unwrap())
    }

    /// The time of when the sun will rise during a given day. (only on daily)
    #[allow(dead_code)]
    pub fn sunrise_time(&self) -> Option<OffsetDateTime> {
        self.sunrise_timestamp
            .map(|x| OffsetDateTime::from_unix_timestamp(x as i64).unwrap())
    }

    /// The time of when the sun will set during a given day. (only on daily)
    #[allow(dead_code)]
    pub fn sunset_time(&self) -> Option<OffsetDateTime> {
        self.sunset_timestamp
            .map(|x| OffsetDateTime::from_unix_timestamp(x as i64).unwrap())
    }
}

/// Object representing the severe weather warnings issued for the requested location by a
/// governmental authority (please see our data sources page for a list of sources).
#[derive(Debug, Serialize, Deserialize)]
pub struct Alert {
    /// A brief description of the alert.
    pub title: String,
    /// A detailed description of the alert.
    pub description: String,
    /// The UNIX time at which the alert was issued.
    #[serde(alias = "time")]
    pub timestamp: u64,
    /// The UNIX time at which the alert will expire.
    #[serde(alias = "expires")]
    pub expires_timestamp: u64,
    /// An array of strings representing the names of the regions covered by this weather alert.
    pub regions: Vec<String>,
    /// The severity of the weather alert. Will take one of the following values: "advisory" (an
    /// individual should be aware of potentially severe weather), "watch" (an individual should
    /// prepare for potentially severe weather), or "warning" (an individual should take immediate
    /// action to protect themselves and others from potentially severe weather).
    pub severity: String,
    /// An HTTP(S) URI that one may refer to for detailed information about the alert.
    pub uri: String,
}

impl Alert {
    /// The time at which the alert was issued.
    #[allow(dead_code)]
    pub fn time(&self) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(self.timestamp as i64).unwrap()
    }

    /// The time at which the alert will expire.
    #[allow(dead_code)]
    pub fn expires(&self) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(self.expires_timestamp as i64).unwrap()
    }
}

/// The flags object contains various metadata information related to the request.
#[derive(Debug, Serialize, Deserialize)]
pub struct Flags {
    // not sure how this is represented in JSON or how Serde will handle it
    //    /// The presence of this property indicates that the Dark Sky data source supports the given
    //    /// location, but a temporary error (such as a radar station being down for maintenance) has
    //    /// made the data unavailable.
    //    #[serde(alias = "darksky-unavailable")]
    //    pub darksky_unavailable: Option<Any>,
    /// The distance to the nearest weather station that contributed data to this response. Note,
    /// however, that many other stations may have also been used; this value is primarily for
    /// debugging purposes. This property's value is in miles (if US units are selected) or
    /// kilometers (if SI units are selected).
    #[serde(alias = "nearest-station")]
    pub nearest_station: f32,
    /// This property contains an array of IDs for each data source utilized in servicing this
    /// request.
    pub sources: Vec<String>,
    /// Indicates the units which were used for the data in this request.
    pub units: String,
}
