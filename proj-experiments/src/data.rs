use proj_models::{network::Flow, RequestEvents};

pub fn load_example_events(path: &str) -> RequestEvents<u32> {
    let file = std::fs::File::open(path).unwrap();
    let reader = std::io::BufReader::new(file);
    bincode::deserialize_from(reader).unwrap()
}

#[allow(dead_code)]
/// Load network traces from a preprocessed events file.
pub fn load_network_trace_events(path: &str) -> RequestEvents<Flow> {
    let file = std::fs::File::open(path).unwrap();
    let reader = std::io::BufReader::new(file);
    bincode::deserialize_from(reader).unwrap()
}
