use time::format_description;
use time::Date;

use std::path::Path;

/// A series of meter readings
#[derive(Debug)]
#[allow(dead_code)]
pub struct Measurements {
    /// The meter readings
    pub data: Vec<Measurement>,
    /// The type of utility being measured (ie, Electricity)
    pub typ: String,
    /// The unit that the measurements are reported in
    pub unit: String,
}

/// A single meter reading
#[derive(Debug)]
pub struct Measurement {
    /// The date of the meter reading
    pub date: Date,
    /// The amount of resources used since the last meter reading
    pub amount: f32,
}

impl Measurement {
    pub fn new(date: Date, amount: f32) -> Measurement {
        Measurement { date, amount }
    }
}

impl Measurements {
    /// Load a measurements object from a CSV file.
    pub fn from_file(path: &Path, typ: String, unit: String) -> Result<Measurements, ReadError> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .trim(csv::Trim::Fields)
            .from_path(path)?;

        let mut records: Vec<Measurement> = Vec::new();
        for result in reader.deserialize() {
            let (date_str, value): (String, u16) = result?;
            let date = Date::parse(
                &date_str,
                &format_description::parse("[year]-[month]-[day]").unwrap(),
            )?;
            records.push(Measurement {
                date,
                amount: f32::from(value),
            })
        }
        records.sort_by(|a, b| a.date.cmp(&b.date));

        Ok(Measurements {
            data: records,
            typ,
            unit,
        })
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum ReadError {
    CsvError { err: csv::Error },
    DateParseError { err: time::error::Parse },
}

impl From<csv::Error> for ReadError {
    fn from(err: csv::Error) -> Self {
        ReadError::CsvError { err }
    }
}

impl From<time::error::Parse> for ReadError {
    fn from(err: time::error::Parse) -> Self {
        ReadError::DateParseError { err }
    }
}
