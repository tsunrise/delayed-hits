use std::{io::Read, time::Duration};

use etherparse::{NetSlice, SlicedPacket, TransportSlice};
use pcap_file::pcap::PcapReader;
use proj_models::{
    network::{Flow, Protocol},
    RequestEvent,
};

pub fn read_flow_from_pcap_data(data: &[u8]) -> Option<Flow> {
    let parsed = SlicedPacket::from_ip(data).ok()?;
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

    Some(Flow {
        src_ip,
        dst_ip,
        src_port,
        dst_port,
        protocol,
    })
}

pub fn read_init_time<R: Read>(reader: R) -> Duration {
    let mut pcap_reader = PcapReader::new(reader).unwrap();
    let first_pkt = pcap_reader.next_packet().unwrap().unwrap();
    first_pkt.timestamp
}

pub fn read_pcap_events<R: Read>(reader: R, init_time: Duration) -> Vec<RequestEvent<Flow>> {
    let mut pcap_reader = PcapReader::new(reader).unwrap();

    let mut events = Vec::new();
    while let Some(pkt) = pcap_reader.next_packet() {
        if let Ok(pkt) = pkt {
            if let Some(flow) = read_flow_from_pcap_data(&pkt.data) {
                events.push((flow, pkt.timestamp));
            }
        }
    }

    // initial timestamp is now always 0
    events
        .iter()
        .map(|(flow, timestamp)| RequestEvent {
            key: flow.clone(),
            timestamp: (*timestamp - init_time).as_nanos() as u64,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{net::Ipv4Addr, path::PathBuf};

    #[test]
    fn test_read_pcap_events() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("test-resources/test.pcap");
        let file = std::fs::File::open(path).unwrap();
        let events = read_pcap_events(file, Duration::from_secs(0));
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
