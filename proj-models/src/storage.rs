use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
/// MSN Storage Server Block ID
pub struct BlockId {
    pub irp_ptr: u64,
    pub disk_num: u32,
}

/// IBM Object Storage Object ID
pub type KVObjectId = u64;
