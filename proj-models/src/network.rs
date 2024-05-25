use std::net::Ipv4Addr;

#[deprecated]
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct Flow {
    pub src_ip: Ipv4Addr,
    pub dst_ip: Ipv4Addr,
    pub src_port: u16,
    pub dst_port: u16,
}
