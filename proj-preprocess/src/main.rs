mod downsample;
mod example_parser;
mod msn_storage_parser;
mod pcap_parser;

use std::io::Write;

use clap::{Parser, Subcommand};
use proj_models::{network::Flow, storage::BlockId, RequestEvent};
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
/// - `initial_path`: the path to the initial pcap file, used to calculate the timestamp offset.
fn read_pcap_traces_from_multiple_files(
    paths: &[&str],
    run_sort: bool,
    initial_path: &str,
) -> Vec<RequestEvent<Flow>> {
    let mut events = Vec::new();
    let init_time = {
        let file = std::fs::File::open(initial_path).unwrap();
        pcap_parser::read_init_time(file)
    };
    for path in paths {
        let file = std::fs::File::open(path).unwrap();
        let mut events_from_file = pcap_parser::read_pcap_events(file, init_time);
        events.append(&mut events_from_file);
    }
    sort_or_check_timestamps(&mut events, run_sort);
    events
}

fn read_storage_traces_from_multiple_files(
    paths: &[&str],
    run_sort: bool,
) -> Vec<RequestEvent<BlockId>> {
    let mut events = Vec::new();
    for path in paths {
        let file = std::fs::File::open(path).unwrap();
        let mut events_from_file = msn_storage_parser::read_msn_storage_events(file);
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

#[derive(Debug, Clone, Subcommand)]
enum SubArgs {
    Example {
        #[clap(short, long)]
        path: String,
        #[clap(short, long)]
        output: String,
    },
    Traces {
        #[clap(
            short,
            long,
            help = "The paths to the pcap files. The first in paths is not necessarily the first pcap file used by the experiment."
        )]
        paths: Vec<String>,
        #[clap(short, long)]
        output: String,
        #[clap(short, long, help = "Let us sort the events by timestamp.")]
        sort: bool,
        #[clap(
            short,
            long,
            help = "The path to the first pcap file used by the experiment. Used to calculate the timestamp offset."
        )]
        initial_path: String,
    },
    ProcessedNetEvents {
        #[clap(short, long)]
        paths: Vec<String>,
        #[clap(short, long)]
        output: String,
        #[clap(short, long)]
        sort: bool,
    },
    StorageTraces {
        #[clap(short, long, help = "The paths to the msn storage traces.")]
        paths: Vec<String>,
        #[clap(short, long)]
        output: String,
        #[clap(short, long)]
        sort: bool,
    },
    DownsampleStorageEvents {
        #[clap(short, long)]
        path: String,
        #[clap(short, long)]
        output: String,
        #[clap(short, long)]
        constant_arrival: bool,
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
        SubArgs::Example { path, output } => {
            let events = read_example_events_from_file(&path);
            write_events_to_binary_file(&events, &output);
        }
        SubArgs::Traces {
            paths,
            output,
            sort,
            initial_path,
        } => {
            let paths = paths.iter().map(|s| s.as_str()).collect::<Vec<_>>();
            let events = read_pcap_traces_from_multiple_files(&paths, sort, &initial_path);
            write_events_to_binary_file(&events, &output);
        }
        SubArgs::ProcessedNetEvents {
            paths,
            output,
            sort,
        } => {
            let paths = paths.iter().map(|s| s.as_str()).collect::<Vec<_>>();
            let events = concat_net_events(&paths, sort);
            write_events_to_binary_file(&events, &output);
        }
        SubArgs::StorageTraces {
            paths,
            output,
            sort,
        } => {
            let paths = paths.iter().map(|s| s.as_str()).collect::<Vec<_>>();
            let events = read_storage_traces_from_multiple_files(&paths, sort);
            write_events_to_binary_file(&events, &output);
        }
        SubArgs::DownsampleStorageEvents {
            path,
            output,
            constant_arrival,
        } => {
            let file = std::fs::File::open(path).unwrap();
            let events: Vec<RequestEvent<BlockId>> = bincode::deserialize_from(file).unwrap();
            let downsampled_events = downsample::downsample_events(events, constant_arrival);
            write_events_to_binary_file(&downsampled_events, &output);
        }
    }
}
