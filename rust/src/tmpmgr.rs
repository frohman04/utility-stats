use crate::darksky::DarkSkyClient;

use chrono::prelude::*;

use std::collections::HashMap;
use std::f32;

pub struct TempDataManager {
    client: DarkSkyClient,
    cache: HashMap<Date<Utc>, Option<Temp>>,
}

impl TempDataManager {
    /// Construct a manager that will use the given client to fetch data
    pub fn new(client: DarkSkyClient) -> TempDataManager {
        TempDataManager {
            client,
            cache: HashMap::new(),
        }
    }

    /// Get the temperature for the provided date
    pub fn get_temp(&mut self, date: Date<Utc>) -> &Option<Temp> {
        if !self.cache.contains_key(&date) {
            let temp = self.fetch_data(date);
            self.cache.insert(date, temp);
        }
        self.cache.get(&date).unwrap()
    }

    /// Fetch the temperature data for the given date.  This data can come from disk cache or direct
    /// from the DarkSky API.
    fn fetch_data(&mut self, date: Date<Utc>) -> Option<Temp> {
        let data = self.client.get_history(date);

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

                Some(Temp {
                    min,
                    mean: sum / count as f32,
                    max,
                })
            } else {
                warn!("No temperature data present for {:?}", date);
                None
            }
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct Temp {
    min: f32,
    mean: f32,
    max: f32,
}
