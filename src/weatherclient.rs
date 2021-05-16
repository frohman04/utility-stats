use time::Date;

pub trait WeatherClient {
    fn get_history(&mut self, date: &Date) -> Option<Temp>;
}

#[derive(Debug, Clone)]
pub struct Temp {
    pub min: f32,
    pub mean: f32,
    pub max: f32,
}
