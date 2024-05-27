//! Message on TCP stream.

use proj_models::{impl_codec, RequestId};

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

impl From<RequestId> for Message {
    fn from(key: RequestId) -> Self {
        Self { key }
    }
}

impl From<Message> for RequestId {
    fn from(msg: Message) -> Self {
        msg.key
    }
}
