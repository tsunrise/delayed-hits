mod cdn_parser;
mod models;
mod ms_prod_parser;
mod msr_storage_parser;
mod pcap_parser;
mod post_process;
use std::{
    io::{BufReader, Write},
    path::PathBuf,
    str::FromStr,
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

fn process_msr_storage(name: &str) {
    let data_root = PathBuf::from_str("data").unwrap().join("msr_cambridge");
    let csv_path = data_root
        .join("raw")
        .join("MSR-Cambridge-2")
        .join(format!("{}.csv", name));
    let output_path = data_root.join(format!("{}.processed.events", name));

    let reader = std::fs::File::open(&csv_path).expect(&format!("Cannot open file {:?}", csv_path));
    let reader = std::io::BufReader::new(reader);
    let raw_requests = msr_storage_parser::read_msr_cambridge_requests(reader);

    let output_file = std::fs::File::create(output_path).unwrap();
    let mut writer = std::io::BufWriter::new(output_file);
    post_process_requests(raw_requests, &mut writer).unwrap();
    writer.flush().unwrap();
}

fn process_cdn_traces() {
    let data_root = PathBuf::from_str("data").unwrap().join("cdn-traces");
    let trace_path = data_root.join("cdn_long.csv");
    let output_path = data_root.join("cdn_long.processed.events");

    let reader =
        std::fs::File::open(&trace_path).expect(&format!("Cannot open file {:?}", trace_path));
    let reader = std::io::BufReader::new(reader);
    let raw_requests = cdn_parser::read_cdn_requests(reader);

    let output_file = std::fs::File::create(output_path).unwrap();
    let mut writer = std::io::BufWriter::new(output_file);
    post_process_requests(raw_requests, &mut writer).unwrap();
    writer.flush().unwrap();
}

fn process_ms_prod_traces() {
    let data_root = PathBuf::from_str("data").unwrap().join("ms_prod");
    // enumerate all files in the directory
    let paths = std::fs::read_dir(&data_root.join("BuildServer").join("Traces"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    let raw_requests = ms_prod_parser::parse_files(paths);

    let output_path = data_root.join("processed.events");
    let output_file = std::fs::File::create(output_path).unwrap();
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
    MsrTraces {
        // positional command
        #[clap(required = true, help = "the trace name")]
        name: String,
    },
    CdnTraces,
    MsProdTraces,
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
        SubArgs::MsrTraces { name } => {
            process_msr_storage(&name);
        }
        SubArgs::CdnTraces => {
            process_cdn_traces();
        }
        SubArgs::MsProdTraces => {
            process_ms_prod_traces();
        }
    }
}
