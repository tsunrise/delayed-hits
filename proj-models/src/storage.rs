use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct BlockId {
    pub irp_ptr: u64,
    pub disk_num: u32,
}
