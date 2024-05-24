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
}

pub type StdObjectId = u64;
