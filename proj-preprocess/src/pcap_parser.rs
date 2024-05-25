use std::{
    io::{BufRead, Read},
    net::Ipv4Addr,
    time::Duration,
};

use etherparse::{NetSlice, SlicedPacket, TransportSlice};
use pcap_file::pcap::PcapReader;

use crate::models::RawRequestWithTimestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Flow {
    pub src_ip: Ipv4Addr,
    pub dst_ip: Ipv4Addr,
    pub src_port: u16,
    pub dst_port: u16,
}

fn read_flow_from_pcap_data(data: &[u8]) -> Option<Flow> {
    let parsed = SlicedPacket::from_ip(data).ok()?;
    let (src_ip, dst_ip) = match parsed.net {
        Some(NetSlice::Ipv4(ipv4)) => (
            ipv4.header().source_addr(),
            ipv4.header().destination_addr(),
        ),
        _ => return None,
    };

    let (src_port, dst_port) = match parsed.transport {
        Some(TransportSlice::Tcp(tcp)) => (tcp.source_port(), tcp.destination_port()),
        Some(TransportSlice::Udp(udp)) => (udp.source_port(), udp.destination_port()),
        _ => return None,
    };

    Some(Flow {
        src_ip,
        dst_ip,
        src_port,
        dst_port,
    })
}

pub fn read_pcap<R: Read>(reader: R) -> Vec<Option<Flow>> {
    let mut pcap_reader = PcapReader::new(reader).unwrap();

    let mut events = Vec::new();
    while let Some(pkt) = pcap_reader.next_packet() {
        if let Ok(pkt) = pkt {
            events.push(read_flow_from_pcap_data(&pkt.data));
        }
    }

    events
}

pub fn read_timestamps<R: BufRead>(reader: R) -> Vec<Duration> {
    reader
        .lines()
        .map(|line| {
            let line = line.unwrap(); // example: 1395323520.000002444
            let mut it = line.split('.');
            let sec = it.next().unwrap().parse::<u64>().unwrap();
            let nsec = it.next().unwrap().parse::<u32>().unwrap();
            debug_assert!(it.next().is_none());
            Duration::new(sec, nsec)
        })
        .collect()
}

pub fn read_pcap_with_timestamps<R: Read, R2: BufRead>(
    pcap_reader: R,
    timestamp_reader: R2,
) -> Vec<RawRequestWithTimestamp<Flow>> {
    let flows = read_pcap(pcap_reader);
    let timestamps = read_timestamps(timestamp_reader);

    assert_eq!(
        flows.len(),
        timestamps.len(),
        "The number of flows and timestamps should match."
    );

    flows
        .into_iter()
        .zip(timestamps)
        .filter_map(|(flow, timestamp)| flow.map(|flow| (flow, timestamp).into()))
        .collect()
}
