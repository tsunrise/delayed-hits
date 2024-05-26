mod data;
use std::fmt::Display;

use clap::{Parser, Subcommand};
use proj_cache_sim::{
    cache::{construct_k_way_cache, lru::LRU, lru_mad::LRUMinAD},
    heuristics,
    simulator::{compute_statistics, run_simulation},
};

#[derive(Debug, Subcommand)]
enum Experiment {
    Trace {
        #[clap(long, short = 'p')]
        event_path: String,
        #[clap(long, short = 'k')]
        cache_counts: usize,
        #[clap(long, short = 'c')]
        cache_capacity: usize,
        #[clap(long, short = 'l', help = "miss latency in nanoseconds")]
        miss_latency: u64,
        #[clap(
            long,
            short = 'w',
            help = "number of warmup requests",
            default_value = "0"
        )]
        warmup: usize,
    },
    Analysis {
        #[clap(required = true)]
        event_path: String,
    },
}

fn analyze_event(event_path: &str) {
    let requests = data::load_data(event_path).collect::<Vec<_>>();
    let maximum_active_objects = heuristics::maximum_active_objects(&requests);
    let mean_irt = heuristics::mean_irt(&requests);

    println!("Number of requests: {}", requests.len());
    println!("Maximum active objects: {}", maximum_active_objects);
    println!(
        "Mean inter-request time: {:.0} ns ({:.2} us)",
        mean_irt,
        mean_irt / 1000.0
    );

    let suggested_total_cache_size = (maximum_active_objects as f64 * 0.05).ceil() as usize;
    println!(
        "Suggested total cache size (0.05): {}",
        suggested_total_cache_size
    );
}

#[derive(Debug, Clone)]
struct ExperimentResult {
    total_latency_lru: u128,
    average_latency_lru: f64,
    total_latency_lru_mad: u128,
    average_latency_lru_mad: f64,
    improvement: f64,
    cache_counts: usize,
    cache_capacity: usize,
    miss_latency: u64,
}

impl Display for ExperimentResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "k: {}, c: {}, miss_latency: {}\n\
            total latency (lru): {}\n\
            average latency (lru): {}\n\
            total latency (lru-mad): {}\n\
            average latency (lru-mad): {}\n\
            improvement (%): {}\n\
            CSV: {}, {}, {}, {}, {}",
            self.cache_counts,
            self.cache_capacity,
            self.miss_latency,
            self.total_latency_lru,
            self.average_latency_lru,
            self.total_latency_lru_mad,
            self.average_latency_lru_mad,
            self.improvement * 100.0,
            self.cache_counts,
            self.cache_capacity,
            self.miss_latency,
            self.average_latency_lru,
            self.average_latency_lru_mad
        )
    }
}

/// Run an experiment with the given parameters.
/// - `requests_path`: the path to the file containing the requests
/// - `cache_counts`: the number of caches in the cache hierarchy
/// - `cache_capacity`: the capacity of each cache
/// - `miss_latency`: the latency of a cache miss
/// - `warmup`: the number of requests to warm up the cache. The warmup requests are not included in the statistics.
fn run_experiment(
    requests_path: &str,
    cache_counts: usize,
    cache_capacity: usize,
    miss_latency: u64,
    warmup: usize,
) -> ExperimentResult {
    let mut lru = construct_k_way_cache(cache_counts, |_| LRU::new(cache_capacity));
    let request_results_lru =
        run_simulation(&mut lru, data::load_data(requests_path), miss_latency);

    let stats = compute_statistics(&request_results_lru);
    let lru_avg_latency = stats.average_latency;

    let mut lru_mad = construct_k_way_cache(cache_counts, |_| {
        LRUMinAD::new(cache_capacity, miss_latency)
    });
    let request_results_lru_mad =
        run_simulation(&mut lru_mad, data::load_data(requests_path), miss_latency);

    let stats = compute_statistics(&request_results_lru_mad[warmup..]);
    let lru_mad_avg_latency = stats.average_latency;

    let improvement = (lru_avg_latency - lru_mad_avg_latency) / lru_avg_latency;

    ExperimentResult {
        total_latency_lru: stats.total_latency,
        average_latency_lru: lru_avg_latency,
        total_latency_lru_mad: stats.total_latency,
        average_latency_lru_mad: lru_mad_avg_latency,
        improvement,
        cache_counts,
        cache_capacity,
        miss_latency,
    }
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
        Experiment::Trace {
            event_path,
            cache_counts,
            cache_capacity,
            miss_latency,
            warmup,
        } => {
            let result = run_experiment(
                &event_path,
                cache_counts,
                cache_capacity,
                miss_latency,
                warmup,
            );
            println!("{}", result);
        }
        Experiment::Analysis { event_path } => {
            analyze_event(&event_path);
        }
    }
}
