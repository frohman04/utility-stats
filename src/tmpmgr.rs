use crate::darksky::DarkSkyClient;

use chrono::prelude::*;
use time::Duration;

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

    /// Generate a range of dates [inclusive start, exclusive end)
    pub fn date_range(start_date: Date<Utc>, end_date: Date<Utc>) -> Vec<Date<Utc>> {
        let mut dates: Vec<Date<Utc>> = Vec::new();
        for i in 0..end_date.signed_duration_since(start_date).num_days() {
            dates.push(start_date.checked_add_signed(Duration::days(i)).unwrap());
        }
        dates
    }

    /// Get the temperature for the provided date
    pub fn get_temp(&mut self, date: &Date<Utc>) -> &Option<Temp> {
        if !self.cache.contains_key(date) {
            let temp = self.fetch_data(date);
            self.cache.insert(*date, temp);
        }
        self.cache.get(date).unwrap()
    }

    /// Get the average temperature over a range of days, using each day's minimum temperature in
    /// Farenheit as the data point to average.
    #[allow(dead_code)]
    pub fn get_avg_min_temp(&mut self, from_date: Date<Utc>, to_date: Date<Utc>) -> f32 {
        self.get_avg_temp(from_date, to_date, &|x: &Temp| x.min)
    }

    /// Get the average temperature over a range of days, using each day's mean temperature in
    /// Farenheit as the data point to average.
    #[allow(dead_code)]
    pub fn get_avg_mean_temp(&mut self, from_date: Date<Utc>, to_date: Date<Utc>) -> f32 {
        self.get_avg_temp(from_date, to_date, &|x: &Temp| x.mean)
    }

    /// Get the average temperature over a range of days, using each day's maximum temperature in
    /// Farenheit as the data point to average.
    #[allow(dead_code)]
    pub fn get_avg_max_temp(&mut self, from_date: Date<Utc>, to_date: Date<Utc>) -> f32 {
        self.get_avg_temp(from_date, to_date, &|x: &Temp| x.max)
    }

    /// Get the average temperature over a range of days, using each day's temp as selected by
    /// selector as the data point to average.
    #[allow(dead_code)]
    fn get_avg_temp(
        &mut self,
        from_date: Date<Utc>,
        to_date: Date<Utc>,
        selector: &dyn Fn(&Temp) -> f32,
    ) -> f32 {
        let temps: Vec<f32> = TempDataManager::date_range(from_date, to_date)
            .iter()
            .map(|date| {
                let temp = self.get_temp(&date);
                let temp = temp.as_ref().unwrap();
                selector(temp)
            })
            .collect();
        temps.iter().sum::<f32>() / temps.len() as f32
    }

    /// Fetch the temperature data for the given date.  This data can come from disk cache or direct
    /// from the DarkSky API.
    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn fetch_data(&mut self, date: &Date<Utc>) -> Option<Temp> {
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

#[derive(Debug, Clone)]
pub struct Temp {
    pub min: f32,
    pub mean: f32,
    pub max: f32,
}
