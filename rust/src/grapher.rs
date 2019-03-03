use crate::measurement::Measurement;
use crate::measurement::Measurements;
use crate::regression::SimpleRegression;
use chrono::Date;
use chrono::Utc;

/// Graph all measurements against smoothed temperatures over the same timeframe
pub fn graph_all(electric_data: Measurements, gas_data: Measurements, loess_days: u8) -> () {
    let mut measurement_dates: Vec<Date<Utc>> = Vec::new();

    for record in &electric_data.data {
        measurement_dates.push(record.date);
    }
    for record in &gas_data.data {
        measurement_dates.push(record.date);
    }
    measurement_dates.sort();
    measurement_dates.dedup();

    let electric_plot_data = get_plot_data(electric_data.data);
    let gas_plot_data = get_plot_data(gas_data.data);
    println!("{:?}", electric_plot_data);
    println!("{:?}", gas_plot_data);
}

/// Convert a series of measurements into points for a scatter plot
fn get_plot_data(data: Vec<Measurement>) -> (Vec<Date<Utc>>, Vec<f32>) {
    let mut dates: Vec<Date<Utc>> = Vec::new();
    let mut amounts: Vec<f32> = Vec::new();

    for i in 1..data.len() {
        let prev = &data[i - 1];
        let curr = &data[i];

        dates.push(curr.date);

        let days = curr.date.signed_duration_since(prev.date).num_days();
        amounts.push(curr.amount as f32 / days as f32);
    }

    (dates, amounts)
}
