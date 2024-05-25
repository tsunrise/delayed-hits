mod models;
mod pcap_parser;
mod post_process;
use std::{
    io::{BufReader, Write},
    path::PathBuf,
};

use clap::{Parser, Subcommand};
use post_process::post_process_requests;

fn process_pcaps(path: &str) {
    let toml_str = std::fs::read_to_string(path).unwrap();
    let config: models::NetTraceExperimentConfig = toml::from_str(&toml_str).unwrap();

    let data_base_path = PathBuf::from(path.trim_end_matches(".toml").to_string());
    let raw_file_path = data_base_path.join("raw");
    let processed_file_path = data_base_path.join("processed.events");

    let pcap_readers = config
        .traces
        .iter()
        .map(|trace| raw_file_path.join(config.prefix.clone() + trace + ".UTC.anon.pcap"))
        .map(|path| std::fs::File::open(path).unwrap());
    let times_readers = config
        .traces
        .iter()
        .map(|trace| raw_file_path.join(config.prefix.clone() + trace + ".UTC.anon.times"))
        .map(|path| {
            let reader = std::fs::File::open(path).unwrap();
            BufReader::new(reader)
        });
    let raw_requests = pcap_readers.zip(times_readers).enumerate().flat_map(
        |(idx, (pcap_reader, timestamp_reader))| {
            let result = pcap_parser::read_pcap_with_timestamps(pcap_reader, timestamp_reader);
            println!("Trace {} has {} requests", idx, result.len());
            result
        },
    );

    let output_file = std::fs::File::create(processed_file_path).unwrap();
    let mut writer = std::io::BufWriter::new(output_file);
    post_process_requests(raw_requests, &mut writer).unwrap();
    writer.flush().unwrap();
}

#[derive(Debug, Clone, Subcommand)]
enum SubArgs {
    NetTraces {
        // positional command
        #[clap(required = true, help = "Path to the toml file for experiment")]
        path: String,
    },
}

#[derive(Debug, Clone, Parser)]
struct Args {
    #[clap(subcommand)]
    args: SubArgs,
}

fn main() {
    let args = Args::parse();
    match args.args {
        SubArgs::NetTraces { path } => {
            process_pcaps(&path);
        }
    }
}
