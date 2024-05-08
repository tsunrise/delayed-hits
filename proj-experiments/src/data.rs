use proj_models::RequestEvents;

pub fn load_example_events(path: &str) -> RequestEvents<u32> {
    let file = std::fs::File::open(path).unwrap();
    let reader = std::io::BufReader::new(file);
    bincode::deserialize_from(reader).unwrap()
}

#[allow(dead_code)]
/// Load network traces from a pcap file.
/// Each object ID represents a flow, and the tuple represents (src, dst, port).
pub fn load_network_traces(path: &str) -> RequestEvents<(u32, u32, u16)> {
    let file = std::fs::File::open(path).unwrap();
    let reader = std::io::BufReader::new(file);
    bincode::deserialize_from(reader).unwrap()
}