//! Message on TCP stream.

use proj_models::{codec::Codec, impl_codec, RequestId};

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub struct CdnRequestMessage {
    pub key: RequestId,
}

impl CdnRequestMessage {
    pub fn new(key: RequestId) -> Self {
        Self { key }
    }
}

impl_codec!(CdnRequestMessage, key, RequestId);

impl From<RequestId> for CdnRequestMessage {
    fn from(key: RequestId) -> Self {
        Self { key }
    }
}

impl From<CdnRequestMessage> for RequestId {
    fn from(msg: CdnRequestMessage) -> Self {
        msg.key
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct FixedSizeResponsePayload<const N: usize> {
    pub content: [u8; N],
}

impl<const N: usize> FixedSizeResponsePayload<N> {
    pub fn new(content: [u8; N]) -> Self {
        Self { content }
    }
}

impl<const N: usize> Codec for FixedSizeResponsePayload<N> {
    type Deserialized = Self;

    const SIZE_IN_BYTES: proj_models::codec::CodecSize = proj_models::codec::CodecSize::Static(N);

    fn size_in_bytes(&self) -> usize {
        N
    }

    fn to_bytes<W: std::io::prelude::Write>(&self, mut writer: W) -> std::io::Result<()> {
        writer.write_all(&self.content)
    }

    fn from_bytes<R: std::io::prelude::Read>(mut reader: R) -> std::io::Result<Self::Deserialized> {
        let mut payload = [0; N];
        reader.read_exact(&mut payload)?;
        Ok(Self { content: payload })
    }
}

impl<const N: usize> Default for FixedSizeResponsePayload<N> {
    fn default() -> Self {
        Self { content: [0; N] }
    }
}

const PAYLOAD_SIZE: usize = 8; // TODO: 250000 * (8+8) is much faster than 500000 * (8+0)??

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct OriginResponseMessage {
    pub key: RequestId,
    // 1KB payload
    pub payload: FixedSizeResponsePayload<PAYLOAD_SIZE>,
}

impl OriginResponseMessage {
    pub fn new(key: RequestId, payload: FixedSizeResponsePayload<PAYLOAD_SIZE>) -> Self {
        Self { key, payload }
    }
}

impl_codec!(
    OriginResponseMessage,
    key,
    RequestId,
    payload,
    FixedSizeResponsePayload<PAYLOAD_SIZE>
);
