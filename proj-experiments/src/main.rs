use clap::{Parser, Subcommand};
use proj_cache_sim::{
    cache::{construct_k_way_cache, lru::LRU, ObjectId},
    simulator::{compute_statistics, run_simulation},
};
use proj_models::RequestEvents;

mod data;

fn run_experiment<K>(
    events: RequestEvents<K>,
    cache_counts: usize,
    cache_capacity: usize,
    miss_latency: u64,
) where
    K: ObjectId,
{
    let mut lru = construct_k_way_cache(cache_counts, |_| LRU::new(cache_capacity));
    let request_results_lru = run_simulation(&mut lru, events.to_simulation_events(), miss_latency);

    let stats = compute_statistics(&request_results_lru);
    println!("average latency (lru): {}", stats.average_latency);

    let mut lru_mad = construct_k_way_cache(cache_counts, |_| {
        proj_cache_sim::cache::lru_mad::LRUMinAD::new(cache_capacity, miss_latency)
    });
    let request_results_lru_mad =
        run_simulation(&mut lru_mad, events.to_simulation_events(), miss_latency);

    let stats = compute_statistics(&request_results_lru_mad);
    println!("average latency (lru-mad): {}", stats.average_latency);
}

fn sanity_check_using_example(
    example_path: &str,
    cache_counts: usize,
    cache_capacity: usize,
    miss_latency: u64,
) {
    let example_events = data::load_example_events(example_path);
    run_experiment(example_events, cache_counts, cache_capacity, miss_latency);
}

fn experiment_using_trace(
    trace_events_path: &str,
    cache_counts: usize,
    cache_capacity: usize,
    miss_latency: u64,
) {
    let trace_events = data::load_network_trace_events(trace_events_path);
    run_experiment(trace_events, cache_counts, cache_capacity, miss_latency);
}

fn analyze_network_trace(trace_events_path: &str) {
    let trace_events = data::load_network_trace_events(trace_events_path);
    let max_active_objects =
        proj_cache_sim::heuristics::maximum_active_objects(&trace_events.events);
    println!("max active objects: {}", max_active_objects);
    let ratios = [0.05];
    for ratio in ratios.iter() {
        println!(
            "cache size for ratio {}: {}",
            ratio,
            (max_active_objects as f64 * ratio) as usize
        );
    }
}

#[derive(Debug, Subcommand)]
enum Experiment {
    SanityCheck {
        #[clap(long, short = 'p')]
        example_path: String,
        #[clap(long, short = 'k')]
        cache_counts: usize,
        #[clap(long, short = 'c')]
        cache_capacity: usize,
        #[clap(long, short = 'l')]
        miss_latency: u64,
    },
    NetworkTrace {
        #[clap(long, short = 'p')]
        events_path: String,
        #[clap(long, short = 'k')]
        cache_counts: usize,
        #[clap(long, short = 'c')]
        cache_capacity: usize,
        #[clap(long, short = 'l')]
        miss_latency: u64,
    },
    NetworkTraceAnalysis {
        #[clap(long, short = 'p')]
        events_path: String,
    },
}

#[derive(Parser, Debug)]
#[command(about = "Run various experiments.")]
struct Args {
    #[clap(subcommand)]
    experiment: Experiment,
}

fn main() {
    let args = Args::parse();
    match args.experiment {
        Experiment::SanityCheck {
            example_path,
            cache_counts,
            cache_capacity,
            miss_latency,
        } => {
            sanity_check_using_example(&example_path, cache_counts, cache_capacity, miss_latency);
        }
        Experiment::NetworkTrace {
            events_path,
            cache_counts,
            cache_capacity,
            miss_latency,
        } => {
            experiment_using_trace(&events_path, cache_counts, cache_capacity, miss_latency);
        }
        Experiment::NetworkTraceAnalysis { events_path } => {
            analyze_network_trace(&events_path);
        }
    }
}
