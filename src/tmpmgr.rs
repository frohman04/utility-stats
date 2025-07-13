use crate::client::{Temp, WeatherClient};

use time::{Date, Duration};

use std::collections::HashMap;
use std::f32;

pub struct TempDataManager {
    clients: Vec<Box<dyn WeatherClient>>,
    cache: HashMap<Date, Option<Temp>>,
}

impl TempDataManager {
    /// Construct a manager that will use the given client to fetch data
    pub fn new(clients: Vec<Box<dyn WeatherClient>>) -> TempDataManager {
        TempDataManager {
            clients,
            cache: HashMap::new(),
        }
    }

    /// Generate a range of dates [inclusive start, exclusive end)
    pub fn date_range(start_date: Date, end_date: Date) -> Vec<Date> {
        let mut dates: Vec<Date> = Vec::new();
        for i in 0..(end_date - start_date).whole_days() {
            dates.push(start_date + Duration::days(i));
        }
        dates
    }

    /// Get the temperature for the provided date
    pub fn get_temp(&mut self, date: &Date) -> &Option<Temp> {
        if !self.cache.contains_key(date) {
            let temps: Vec<Option<Temp>> = self
                .clients
                .iter_mut()
                .map(|client| client.get_history(date))
                .collect();

            let mut min: f32 = f32::MAX;
            let mut max: f32 = f32::MIN;
            let mut mean_sum: f32 = 0f32;
            let mut count: u8 = 0;
            for temp in temps {
                temp.iter().for_each(|t| {
                    min = min.min(t.min);
                    max = max.max(t.max);
                    mean_sum += t.mean;
                    count += 1;
                })
            }
            let temp = if count > 0 {
                Some(Temp {
                    min,
                    max,
                    mean: mean_sum / count as f32,
                })
            } else {
                None
            };

            self.cache.insert(*date, temp);
        }
        self.cache.get(date).unwrap()
    }

    /// Get the average temperature over a range of days, using each day's minimum temperature in
    /// Farenheit as the data point to average.
    #[allow(dead_code)]
    pub fn get_avg_min_temp(&mut self, from_date: Date, to_date: Date) -> f32 {
        self.get_avg_temp(from_date, to_date, &|x: &Temp| x.min)
    }

    /// Get the average temperature over a range of days, using each day's mean temperature in
    /// Farenheit as the data point to average.
    #[allow(dead_code)]
    pub fn get_avg_mean_temp(&mut self, from_date: Date, to_date: Date) -> f32 {
        self.get_avg_temp(from_date, to_date, &|x: &Temp| x.mean)
    }

    /// Get the average temperature over a range of days, using each day's maximum temperature in
    /// Farenheit as the data point to average.
    #[allow(dead_code)]
    pub fn get_avg_max_temp(&mut self, from_date: Date, to_date: Date) -> f32 {
        self.get_avg_temp(from_date, to_date, &|x: &Temp| x.max)
    }

    /// Get the average temperature over a range of days, using each day's temp as selected by
    /// selector as the data point to average.
    #[allow(dead_code)]
    fn get_avg_temp(
        &mut self,
        from_date: Date,
        to_date: Date,
        selector: &dyn Fn(&Temp) -> f32,
    ) -> f32 {
        let temps: Vec<f32> = TempDataManager::date_range(from_date, to_date)
            .iter()
            .map(|date| {
                let temp = self.get_temp(date);
                let temp = temp.as_ref().unwrap();
                selector(temp)
            })
            .collect();
        temps.iter().sum::<f32>() / temps.len() as f32
    }
}
