use std::net::Ipv4Addr;

use derivative::Derivative;
use serde_derive::{Deserialize, Serialize};

#[derive(Eq, PartialEq, Hash, Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Protocol {
    TCP,
    UDP,
}

#[derive(Derivative, Debug, Clone, Copy, Serialize, Deserialize)]
#[derivative(Hash, Eq, PartialEq)]
pub struct Flow {
    pub src_ip: Ipv4Addr,
    pub dst_ip: Ipv4Addr,
    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub src_port: u16,
    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub dst_port: u16,
    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub protocol: Protocol,
}
