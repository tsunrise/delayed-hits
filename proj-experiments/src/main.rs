use clap::{Parser, Subcommand};
use proj_cache_sim::{
    cache::{construct_k_way_cache, lru::LRU},
    simulator::{compute_statistics, run_simulation},
};

mod data;

fn sanity_check_using_example(
    example_path: &str,
    cache_counts: usize,
    cache_capacity: usize,
    miss_latency: usize,
) {
    let example_events = data::load_example_events(example_path);

    let mut lru = construct_k_way_cache(cache_counts, |_| LRU::new(cache_capacity));
    let request_results_lru = run_simulation(
        &mut lru,
        example_events.to_simulation_events(),
        miss_latency,
    );

    let stats = compute_statistics(&request_results_lru);
    println!("average latency (lru): {}", stats.average_latency);

    let mut lru_mad = construct_k_way_cache(cache_counts, |_| {
        proj_cache_sim::cache::lru_mad::LRUMinAD::new(cache_capacity, miss_latency as u64)
    });
    let request_results_lru_mad = run_simulation(
        &mut lru_mad,
        example_events.to_simulation_events(),
        miss_latency,
    );

    let stats = compute_statistics(&request_results_lru_mad);
    println!("average latency (lru-mad): {}", stats.average_latency);
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
        miss_latency: usize,
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
    }
}
