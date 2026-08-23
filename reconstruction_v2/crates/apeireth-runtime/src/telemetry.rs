pub struct Telemetry;

impl Telemetry {
    pub fn record_latency(_ms: u64) {}
    pub fn record_span(_name: &str) {}
}
