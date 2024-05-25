use std::time::Duration;

use serde_derive::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct NetTraceExperimentConfig {
    pub url: String,
    pub prefix: String,
    pub traces: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct RawRequestWithTimestamp<T> {
    pub request: T,
    pub timestamp: Duration,
}

impl<T> From<(T, Duration)> for RawRequestWithTimestamp<T> {
    fn from((request, timestamp): (T, Duration)) -> Self {
        Self { request, timestamp }
    }
}
