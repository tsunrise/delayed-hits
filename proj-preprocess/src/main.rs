mod example_parser;
mod msn_storage_parser;
mod pcap_parser;

use std::io::Write;

use clap::{Parser, ValueEnum};
use proj_models::{network::Flow, RequestEvent};
use serde::Serialize;

fn read_example_events_from_file(path: &str) -> Vec<RequestEvent<u32>> {
    let file = std::fs::File::open(path).unwrap();
    example_parser::read_example_events(file)
}

fn sort_or_check_timestamps<K>(events: &mut Vec<RequestEvent<K>>, run_sort: bool) {
    if run_sort {
        events.sort_by_key(|event| event.timestamp);
    } else {
        assert!(
            events
                .windows(2)
                .all(|pair| pair[0].timestamp <= pair[1].timestamp),
            "Events are not sorted by timestamp, make sure they are sorted or set `--sort` flag to let us sort them for you."
        );
    }
}

/// - `paths`: paths to the pcap files
/// - `run_sort`: whether to sort the events by timestamp. If `false`, you assume the paths are already sorted by timestamp.
fn read_pcap_traces_from_multiple_files(paths: &[&str], run_sort: bool) -> Vec<RequestEvent<Flow>> {
    let mut events = Vec::new();
    for path in paths {
        let file = std::fs::File::open(path).unwrap();
        let mut events_from_file = pcap_parser::read_pcap_events(file);
        events.append(&mut events_from_file);
    }
    sort_or_check_timestamps(&mut events, run_sort);
    events
}

fn concat_net_events(paths: &[&str], run_sort: bool) -> Vec<RequestEvent<Flow>> {
    let mut events = Vec::new();
    for path in paths {
        let file = std::fs::File::open(path).unwrap();
        let mut events_from_file = bincode::deserialize_from(file).unwrap();
        events.append(&mut events_from_file);
    }
    sort_or_check_timestamps(&mut events, run_sort);
    events
}

fn write_events_to_binary_file<K>(events: &[RequestEvent<K>], path: &str)
where
    K: Serialize,
{
    let file = std::fs::File::create(path).unwrap();
    let mut writer = std::io::BufWriter::new(file);
    bincode::serialize_into(&mut writer, events).unwrap();
    writer.flush().unwrap();
}

#[derive(ValueEnum, Debug, Clone)]
enum RunType {
    Example,
    Traces,
    ProcessedNetEvents,
}

#[derive(Parser, Debug)]
#[command(about = "Preprocess the example or traces.")]
struct Args {
    #[arg(value_enum, short, long)]
    ftype: RunType,
    #[arg(short, long)]
    paths: Vec<String>,
    #[arg(short, long)]
    output: String,
    #[arg(short, long)]
    sort: bool,
}

fn main() {
    // <binary> --ftype example --paths path/to/example --output path/to/output
    // <binary> --ftype traces --paths path/to/traces --output path/to/output

    let args = Args::parse();
    let paths = args.paths.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    match args.ftype {
        RunType::Example => {
            assert!(args.paths.len() == 1);
            let events = read_example_events_from_file(paths[0]);
            write_events_to_binary_file(&events, &args.output);
        }
        RunType::Traces => {
            let events = read_pcap_traces_from_multiple_files(&paths, args.sort);
            write_events_to_binary_file(&events, &args.output);
        }
        RunType::ProcessedNetEvents => {
            let events = concat_net_events(&paths, args.sort);
            write_events_to_binary_file(&events, &args.output);
        }
    }
}
