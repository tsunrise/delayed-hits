//! Message on TCP stream.

use proj_models::{impl_codec, RequestId};
use tokio::io::AsyncRead;

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
