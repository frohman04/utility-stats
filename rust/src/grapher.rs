use crate::measurement::Measurement;
use crate::measurement::Measurements;
use crate::regression::SimpleRegression;
use crate::tmpmgr::TempDataManager;

use chrono::prelude::*;
use time::Duration;

use crate::tmpmgr::Temp;
use std::fs::write;

/// Graph all measurements against smoothed temperatures over the same timeframe
pub fn graph_all(
    electric_data: Measurements,
    gas_data: Measurements,
    mgr: &mut TempDataManager,
    loess_days: u8,
) -> () {
    let mut measurement_dates: Vec<Date<Utc>> = Vec::new();

    for record in &electric_data.data {
        measurement_dates.push(record.date);
    }
    for record in &gas_data.data {
        measurement_dates.push(record.date);
    }
    measurement_dates.sort();
    measurement_dates.dedup();

    let daily_temp_data: Vec<(Date<Utc>, Temp)> =
        TempDataManager::date_range(measurement_dates[0], *measurement_dates.last().unwrap())
            .into_iter()
            .filter_map(|date| mgr.get_temp(&date).clone().map(|temp| (date, temp)))
            .collect();
    let loess_max_temp_plot_data: (Vec<Date<Utc>>, Vec<f32>) = calc_temp_series(
        daily_temp_data
            .iter()
            .map(|(date, temp)| Measurement::new(*date, temp.max))
            .collect(),
        loess_days,
    );
    let loess_min_temp_plot_data: (Vec<Date<Utc>>, Vec<f32>) = calc_temp_series(
        daily_temp_data
            .iter()
            .map(|(date, temp)| Measurement::new(*date, temp.min))
            .collect(),
        loess_days,
    );
    let electric_plot_data = calc_measurement_series(electric_data.data);
    let gas_plot_data = calc_measurement_series(gas_data.data);

    let (loess_max_temp_dates, loess_max_temp_values) =
        to_plot(loess_max_temp_plot_data.0, loess_max_temp_plot_data.1);
    let (loess_min_temp_dates, loess_min_temp_values) =
        to_plot(loess_min_temp_plot_data.0, loess_min_temp_plot_data.1);
    let (electric_dates, electric_values) = to_plot(electric_plot_data.0, electric_plot_data.1);
    let (gas_dates, gas_values) = to_plot(gas_plot_data.0, gas_plot_data.1);

    let html = format!(
        "<!DOCTYPE html>
<html>
    <head>
        <title>All Utilities Usage per Day vs Average {}-day Smoothed Temperature</title>
        <script src=\"https://cdn.plot.ly/plotly-1.41.3.min.js\"></script>
    </head>
    <body>
        <div id=\"chart\"></div>
        <script>
            (function () {{
                var data0 = {{
                    \"name\": \"Max Temp (F)\",
                    \"x\": [{}],
                    \"y\": [{}],
                    \"mode\": \"lines\",
                    \"type\": \"scatter\",
                    \"yaxis\": \"y\"
                }};
                var data1 = {{
                    \"name\": \"Min Temp (F)\",
                    \"x\": [{}],
                    \"y\": [{}],
                    \"mode\": \"lines\",
                    \"type\": \"scatter\",
                    \"yaxis\": \"y\"
                }};
                var data2 = {{
                    \"name\": \"Electric (kWh/day)\",
                    \"x\": [{}],
                    \"y\": [{}],
                    \"mode\": \"lines\",
                    \"type\": \"scatter\",
                    \"yaxis\": \"y2\"
                }};
                var data3 = {{
                    \"name\": \"Gas (CCF/day)\",
                    \"x\": [{}],
                    \"y\": [{}],
                    \"mode\": \"lines\",
                    \"type\": \"scatter\",
                    \"yaxis\": \"y3\"
                }};

                var data = [data0, data1, data2, data3];
                var layout = {{
                    \"title\": \"All Utilities Usage per Day vs Average {}-day Smoothed Temperature\",
                    \"xaxis\": {{
                        \"title\": \"Measurement Date\"
                    }},
                    \"yaxis\": {{
                        \"title\": \"Avg Temp (F)\"
                    }},
                    \"yaxis2\": {{
                        \"showgrid\": false,
                        \"showticklabels\": false,
                        \"overlaying\": \"y\"
                    }},
                    \"yaxis3\": {{
                        \"showgrid\": false,
                        \"showticklabels\": false,
                        \"overlaying\": \"y\"
                    }}
                }};
                Plotly.plot(\"chart\", data, layout);
            }})();
        </script>
    </body>
</html>",
        loess_days,
        loess_max_temp_dates,
        loess_max_temp_values,
        loess_min_temp_dates,
        loess_min_temp_values,
        electric_dates,
        electric_values,
        gas_dates,
        gas_values,
        loess_days
    );

    write("all-utilities.html", html).expect("Unable to write file");
}

/// Convert a series of measurements into points for a scatter plot
fn calc_measurement_series(data: Vec<Measurement>) -> (Vec<Date<Utc>>, Vec<f32>) {
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

/// Convert a series of measurements into smoothed points for a scatter plot
fn calc_temp_series(data: Vec<Measurement>, num_days: u8) -> (Vec<Date<Utc>>, Vec<f32>) {
    let base_date = data.iter().map(|r| r.date).min().unwrap();
    let mut lower_init = 0;

    let mut dates: Vec<Date<Utc>> = Vec::new();
    let mut amounts: Vec<f32> = Vec::new();

    for measurement in &data {
        let lower_bound = measurement.date - Duration::days(num_days as i64 / 2);
        let upper_bound = measurement.date + Duration::days((num_days as i64 - 1) / 2);

        let mut regression = SimpleRegression::new();

        let mut i = lower_init;
        while lower_bound.signed_duration_since(data[i].date).num_days() > 0 {
            i += 1;
        }
        lower_init = i;

        while i < data.len() && data[i].date.signed_duration_since(upper_bound).num_days() <= 0 {
            regression.add_data(
                data[i].date.signed_duration_since(base_date).num_days() as f64,
                data[i].amount as f64,
            );
            i += 1;
        }

        dates.push(measurement.date);
        amounts.push(
            regression.predict(measurement.date.signed_duration_since(base_date).num_days() as f64)
                as f32,
        );
    }

    (dates, amounts)
}

/// Convert a data series into the format for putting into JS.
fn to_plot(dates: Vec<Date<Utc>>, values: Vec<f32>) -> (String, String) {
    let dates: Vec<String> = dates
        .iter()
        .map(|x| x.format("%Y-%m-%d").to_string())
        .collect();
    let values: Vec<String> = values.iter().map(|x| x.to_string()).collect();
    (
        if dates.len() > 0 {
            format!("\"{}\"", dates.join("\",\""))
        } else {
            "".to_string()
        },
        values.join(","),
    )
}
