mod example_parser;
mod pcap_parser;

use std::io::Write;

use clap::{Parser, ValueEnum};
use pcap_parser::Flow;
use proj_models::RequestEvent;
use serde::Serialize;

fn read_example_events_from_file(path: &str) -> Vec<RequestEvent<u32>> {
    let file = std::fs::File::open(path).unwrap();
    example_parser::read_example_events(file)
}

fn read_pcap_traces_from_file(path: &str) -> Vec<RequestEvent<Flow>> {
    let file = std::fs::File::open(path).unwrap();
    pcap_parser::read_pcap_events(file)
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
}

#[derive(Parser, Debug)]
#[command(about = "Preprocess the example or traces.")]
struct Args {
    #[arg(value_enum, short, long)]
    ftype: RunType,
    #[arg(short, long)]
    path: String,
    #[arg(short, long)]
    output: String,
}

fn main() {
    // <binary> --ftype example --path path/to/example --output path/to/output
    // <binary> --ftype traces --path path/to/traces --output path/to/output

    let args = Args::parse();
    match args.ftype {
        RunType::Example => {
            let events = read_example_events_from_file(&args.path);
            write_events_to_binary_file(&events, &args.output);
        }
        RunType::Traces => {
            let events = read_pcap_traces_from_file(&args.path);
            write_events_to_binary_file(&events, &args.output);
        }
    }
}
