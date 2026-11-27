pub trait Clock {
    fn now_iso8601(&self) -> String;
}
pub struct SystemClock;
impl Clock for SystemClock {
    fn now_iso8601(&self) -> String {
        format!(
            "{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        )
    }
}
