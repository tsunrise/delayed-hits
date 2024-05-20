pub mod network;
pub mod storage;

use serde_derive::{Deserialize, Serialize};

use std::fmt::Debug;

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct RequestEvent<K> {
    pub key: K,
    pub timestamp: u64,
}

// TODO: Deserialize to a stream if the trace is too large.
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct RequestEvents<K> {
    pub events: Vec<RequestEvent<K>>,
}

impl<K> RequestEvents<K> {
    pub fn new(events: Vec<RequestEvent<K>>) -> Self {
        Self { events }
    }

    pub fn to_simulation_events(&self) -> impl Iterator<Item = (K, u64)> + '_
    where
        K: Clone,
    {
        self.events
            .iter()
            .map(|event| (event.key.clone(), event.timestamp))
    }

    pub fn into_simulation_events(self) -> impl Iterator<Item = (K, u64)> {
        self.events
            .into_iter()
            .map(|event| (event.key, event.timestamp))
    }
}
