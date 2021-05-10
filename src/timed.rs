#[macro_export]
macro_rules! timed {
    ($msg:expr, $($args:expr)+, $closure:expr) => {{
        use time::OffsetDateTime;
        let msg = format!($msg, $($args)*);

        let start_time = OffsetDateTime::now_utc();
        info!("Start: {}", msg);

        #[allow(clippy::redundant_closure_call)]
        let out = $closure();

        let end_time = OffsetDateTime::now_utc();
        let duration = end_time - start_time;
        info!("End:   {}: {}s", msg, (duration.whole_microseconds() as f64) / 1_000_000f64);

        out
    }};
    ($msg:expr, $closure:expr) => {{
        use time::OffsetDateTime;
        let msg: &str = $msg;

        let start_time = OffsetDateTime::now_utc();
        info!("Start: {}", msg);

        #[allow(clippy::redundant_closure_call)]
        let out = $closure();

        let end_time = OffsetDateTime::now_utc();
        let duration = end_time - start_time;
        info!("End:   {}: {}s", msg, (duration.whole_microseconds() as f64) / 1_000_000f64);

        out
    }};
}
