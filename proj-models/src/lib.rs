pub mod codec;
pub mod storage;

use std::fmt::Debug;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct RequestEvent {
    pub key: RequestId,
    pub timestamp: TimeUnit,
}

impl_codec!(RequestEvent, key, RequestId, timestamp, TimeUnit);

pub type RequestId = u64;
/// timestamp in specified unit (ns)
pub type TimeUnit = u64;
