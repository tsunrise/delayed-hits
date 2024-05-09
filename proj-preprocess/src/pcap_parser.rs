use std::{io::Read, net::Ipv4Addr};

use etherparse::{NetSlice, SlicedPacket, TransportSlice};
use pcap_file::pcap::PcapReader;
use proj_models::RequestEvent;
use serde_derive::{Deserialize, Serialize};

#[derive(Eq, PartialEq, Hash, Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Protocol {
    TCP,
    UDP,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Flow {
    pub src_ip: Ipv4Addr,
    pub dst_ip: Ipv4Addr,
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: Protocol,
}

impl Flow {
    pub fn from_pcap_data(data: &[u8]) -> Option<Self> {
        let parsed = SlicedPacket::from_ethernet(data).ok()?;
        let (src_ip, dst_ip) = match parsed.net {
            Some(NetSlice::Ipv4(ipv4)) => (
                ipv4.header().source_addr(),
                ipv4.header().destination_addr(),
            ),
            _ => return None,
        };

        let (src_port, dst_port, protocol) = match parsed.transport {
            Some(TransportSlice::Tcp(tcp)) => {
                (tcp.source_port(), tcp.destination_port(), Protocol::TCP)
            }
            Some(TransportSlice::Udp(udp)) => {
                (udp.source_port(), udp.destination_port(), Protocol::UDP)
            }
            _ => return None,
        };

        Some(Self {
            src_ip,
            dst_ip,
            src_port,
            dst_port,
            protocol,
        })
    }
}

pub fn read_pcap_events<R: Read>(reader: R) -> Vec<RequestEvent<Flow>> {
    let mut pcap_reader = PcapReader::new(reader).unwrap();

    let mut events = Vec::new();
    while let Some(pkt) = pcap_reader.next_packet() {
        if let Ok(pkt) = pkt {
            if let Some(flow) = Flow::from_pcap_data(&pkt.data) {
                events.push((flow, pkt.timestamp.as_nanos()));
            }
        }
    }

    // initial timestamp is now always 0
    let init_timestamp = events.first().map(|(_, timestamp)| *timestamp).unwrap_or(0);
    events
        .iter()
        .map(|(flow, timestamp)| RequestEvent {
            key: flow.clone(),
            timestamp: (timestamp - init_timestamp) as u64,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_read_pcap_events() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("test-resources/test.pcap");
        let file = std::fs::File::open(path).unwrap();
        let events = read_pcap_events(file);
        assert_eq!(
            events[0].key,
            Flow {
                src_ip: Ipv4Addr::new(172, 16, 11, 12),
                dst_ip: Ipv4Addr::new(74, 125, 19, 17),
                src_port: 64565,
                dst_port: 443,
                protocol: Protocol::TCP,
            }
        );
        assert_eq!(
            events[1].key,
            Flow {
                src_ip: Ipv4Addr::new(172, 16, 11, 12),
                dst_ip: Ipv4Addr::new(74, 125, 19, 17),
                src_port: 64565,
                dst_port: 443,
                protocol: Protocol::TCP,
            }
        );
        assert_eq!(
            events[2].key,
            Flow {
                src_ip: Ipv4Addr::new(74, 125, 19, 17),
                dst_ip: Ipv4Addr::new(172, 16, 11, 12),
                src_port: 443,
                dst_port: 64565,
                protocol: Protocol::TCP,
            }
        )
    }
}
