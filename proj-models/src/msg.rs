//! Message on TCP stream.

use crate::{impl_codec, RequestId};

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub struct Message {
    pub key: RequestId,
}

impl Message {
    pub fn new(key: RequestId) -> Self {
        Self { key }
    }
}

impl_codec!(Message, key, RequestId);
