pub mod codec;
pub mod storage;

use codec::Codec;

use std::{fmt::Debug, io::Read};

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct RequestEvent {
    pub key: RequestId,
    pub timestamp: TimeUnit,
}

impl Codec for RequestEvent {
    type Deserialized = Self;

    fn size_in_bytes(&self) -> usize {
        self.key.size_in_bytes() + self.timestamp.size_in_bytes()
    }

    fn to_bytes<W: std::io::prelude::Write>(&self, mut writer: W) -> std::io::Result<()> {
        self.key.to_bytes(&mut writer)?;
        self.timestamp.to_bytes(&mut writer)
    }

    fn from_bytes<R: Read>(mut reader: R) -> std::io::Result<Self::Deserialized> {
        let key = RequestId::from_bytes(&mut reader)?;
        let timestamp = TimeUnit::from_bytes(&mut reader)?;
        Ok(Self { key, timestamp })
    }
}

pub type RequestId = u64;
/// timestamp in specified unit (ns)
pub type TimeUnit = u64;
