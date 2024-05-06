use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_derive::{Deserialize, Serialize};

use std::fmt::Debug;
use std::hash::Hash;

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestEvent<K>
where
    K: Hash + Eq + PartialEq + Clone + Debug + Serialize + DeserializeOwned,
{
    #[serde(deserialize_with = "K::deserialize")]
    pub key: K,
    pub timestamp: u64,
}

// TODO: Deserialize to a stream if the trace is too large.
#[derive(Debug, Serialize, Deserialize)]
pub struct RequestEvents<K>
where
    K: Hash + Eq + PartialEq + Clone + Debug + Serialize + DeserializeOwned,
{
    #[serde(deserialize_with = "Vec::deserialize")]
    pub events: Vec<RequestEvent<K>>,
}
