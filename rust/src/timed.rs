#[macro_export]
macro_rules! timed {
    ($msg:tt, $closure:tt) => {{
        use time::PreciseTime;
        let msg: &str = $msg;

        let start_time = PreciseTime::now();
        info!("Start: {}", msg);

        let out = $closure();

        let end_time = PreciseTime::now();
        info!("End:   {}: {}", msg, start_time.to(end_time));

        out
    }};
}
