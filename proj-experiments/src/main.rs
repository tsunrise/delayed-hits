use clap::{Parser, Subcommand};
use proj_cache_sim::{
    cache::{construct_k_way_cache, lru::LRU, ObjectId},
    simulator::{compute_statistics, run_simulation},
};
use proj_models::{network::Flow, RequestEvent, RequestEvents};

mod data;

pub trait EventsIterable<K> {
    fn iter_events(&self) -> impl Iterator<Item = RequestEvent<K>> + '_;

    fn iter_simulation_events(&self) -> impl Iterator<Item = (K, u64)> + '_ {
        self.iter_events().map(|event| (event.key, event.timestamp))
    }
}

impl<K: Clone> EventsIterable<K> for RequestEvents<K> {
    fn iter_events(&self) -> impl Iterator<Item = RequestEvent<K>> + '_ {
        self.events.iter().cloned()
    }
}

struct NetworkTraceEventsLoader<'a> {
    paths: &'a [String],
}

impl<'a> EventsIterable<Flow> for NetworkTraceEventsLoader<'a> {
    fn iter_events(&self) -> impl Iterator<Item = RequestEvent<Flow>> + '_ {
        self.paths
            .iter()
            .flat_map(|path| data::load_network_trace_events(path).events)
    }
}

fn run_experiment<K, E>(events: E, cache_counts: usize, cache_capacity: usize, miss_latency: u64)
where
    K: ObjectId,
    E: EventsIterable<K>,
{
    let mut lru = construct_k_way_cache(cache_counts, |_| LRU::new(cache_capacity));
    let request_results_lru =
        run_simulation(&mut lru, events.iter_simulation_events(), miss_latency);

    let stats = compute_statistics(&request_results_lru);
    println!("total latency (lru): {}", stats.total_latency);
    println!("average latency (lru): {}", stats.average_latency);
    let lru_avg_latency = stats.average_latency;

    let mut lru_mad = construct_k_way_cache(cache_counts, |_| {
        proj_cache_sim::cache::lru_mad::LRUMinAD::new(cache_capacity, miss_latency)
    });
    let request_results_lru_mad =
        run_simulation(&mut lru_mad, events.iter_simulation_events(), miss_latency);

    let stats = compute_statistics(&request_results_lru_mad);
    println!("total latency (lru-mad): {}", stats.total_latency);
    println!("average latency (lru-mad): {}", stats.average_latency);
    let lru_mad_avg_latency = stats.average_latency;

    let improvement = (lru_avg_latency - lru_mad_avg_latency) / lru_avg_latency;
    println!("improvement (%): {}", improvement * 100.0);

    println!(
        "CSV: {}, {}, {}, {}, {}",
        cache_counts, cache_capacity, miss_latency, lru_avg_latency, lru_mad_avg_latency
    );
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
    trace_events_path: &[String],
    cache_counts: usize,
    cache_capacity: usize,
    miss_latency: u64,
) {
    let trace_events = NetworkTraceEventsLoader {
        paths: trace_events_path,
    };
    run_experiment(trace_events, cache_counts, cache_capacity, miss_latency);
}

fn analyze_network_trace(trace_events_path: &[String]) {
    let trace_events = NetworkTraceEventsLoader {
        paths: trace_events_path,
    }
    .iter_events()
    .collect::<Vec<_>>();
    let max_active_objects = proj_cache_sim::heuristics::maximum_active_objects(&trace_events);
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
        events_path: Vec<String>,
        #[clap(long, short = 'k')]
        cache_counts: usize,
        #[clap(long, short = 'c')]
        cache_capacity: usize,
        #[clap(long, short = 'l')]
        miss_latency: u64,
    },
    NetworkTraceAnalysis {
        #[clap(long, short = 'p')]
        events_path: Vec<String>,
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
