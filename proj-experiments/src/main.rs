mod data;
mod irt_ratio_experiment;

use derivative::Derivative;
use rand::SeedableRng as _;
use rand_xorshift::XorShiftRng;
use rayon::prelude::*;
use serde::de::DeserializeOwned;
use std::fmt::Display;

use clap::{Parser, Subcommand};
use proj_cache_sim::{
    cache::{construct_k_way_cache, lru::LRU, ObjectId},
    simulator::{compute_statistics, run_simulation},
};
use proj_models::{
    network::Flow,
    storage::{BlockId, KVObjectId},
    RequestEvent, RequestEvents,
};

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

#[derive(Debug, Derivative)]
#[derivative(Copy(bound = ""), Clone(bound = ""))]
struct EventsLoader<'a, T> {
    paths: &'a [String],
    _phantom: std::marker::PhantomData<T>,
}

impl<'a, T> EventsLoader<'a, T> {
    fn new(paths: &'a [String]) -> Self {
        Self {
            paths,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<'a, T> EventsIterable<T> for EventsLoader<'a, T>
where
    T: DeserializeOwned,
{
    fn iter_events(&self) -> impl Iterator<Item = RequestEvent<T>> + '_ {
        self.paths
            .iter()
            .flat_map(|path| data::load_events(path).events)
    }
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

fn run_experiment<K, E>(
    events: E,
    cache_counts: usize,
    cache_capacity: usize,
    miss_latency: u64,
) -> ExperimentResult
where
    K: ObjectId,
    E: EventsIterable<K>,
{
    let mut lru = construct_k_way_cache(cache_counts, |_| LRU::new(cache_capacity));
    let request_results_lru =
        run_simulation(&mut lru, events.iter_simulation_events(), miss_latency);

    let stats = compute_statistics(&request_results_lru);
    let lru_avg_latency = stats.average_latency;

    let mut lru_mad = construct_k_way_cache(cache_counts, |_| {
        proj_cache_sim::cache::lru_mad::LRUMinAD::new(cache_capacity, miss_latency)
    });
    let request_results_lru_mad =
        run_simulation(&mut lru_mad, events.iter_simulation_events(), miss_latency);

    let stats = compute_statistics(&request_results_lru_mad);
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

fn sanity_check_using_example(
    example_path: &str,
    cache_counts: usize,
    cache_capacity: usize,
    miss_latency: u64,
) {
    let example_events = data::load_example_events(example_path);
    run_experiment(example_events, cache_counts, cache_capacity, miss_latency);
}

fn experiment_using_events_path<T>(
    trace_events_path: &[String],
    cache_counts: usize,
    cache_capacity: usize,
    miss_latency: &[u64],
) where
    T: ObjectId + DeserializeOwned + Send + Sync,
{
    let trace_events = EventsLoader::<T>::new(trace_events_path);
    // run_experiment(trace_events, cache_counts, cache_capacity, miss_latency);
    let result = miss_latency
        .par_iter()
        .map(|&miss_latency| {
            run_experiment(trace_events, cache_counts, cache_capacity, miss_latency)
        })
        .collect::<Vec<_>>();

    for r in result {
        println!("{}", r);
        println!()
    }
}

fn analyze_events<T>(trace_events_path: &[String])
where
    T: ObjectId + DeserializeOwned,
{
    let trace_events = EventsLoader::<T>::new(trace_events_path)
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
    let avg_rearrive_interval = proj_cache_sim::heuristics::median_rearrive_interval(&trace_events);
    println!("median rearrive interval: {}", avg_rearrive_interval);
    let irt = proj_cache_sim::heuristics::irt(&trace_events);
    println!("median irt: {}", irt);
    println!(
        "avg rearrive interval / irt: {}",
        avg_rearrive_interval / irt
    );
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
        #[clap(long, short = 'l', help = "miss latency in nanoseconds")]
        miss_latency: Vec<u64>,
    },
    NetworkTraceAnalysis {
        #[clap(long, short = 'p')]
        events_path: Vec<String>,
    },
    StorageTrace {
        #[clap(long, short = 'p')]
        events_path: Vec<String>,
        #[clap(long, short = 'k')]
        cache_counts: usize,
        #[clap(long, short = 'c')]
        cache_capacity: usize,
        #[clap(long, short = 'l', help = "miss latency in microseconds")]
        miss_latency: Vec<u64>,
    },
    IbmKvTrace {
        #[clap(long, short = 'p')]
        events_path: Vec<String>,
        #[clap(long, short = 'k')]
        cache_counts: usize,
        #[clap(long, short = 'c')]
        cache_capacity: usize,
        #[clap(long, short = 'l', help = "miss latency in milliseconds")]
        miss_latency: Vec<u64>,
    },
    StorageTraceAnalysis {
        #[clap(long, short = 'p')]
        events_path: Vec<String>,
    },
    IbmKvTraceAnalysis {
        #[clap(long, short = 'p')]
        events_path: Vec<String>,
    },
    IrtRatioTest {
        #[clap(long, short = 'k')]
        cache_counts: usize,
        #[clap(long, short = 'c')]
        cache_capacity: usize,
        #[clap(long, short = 'l')]
        miss_latency: u64,
        #[clap(long, short = 'r')]
        expected_rearrival_time: usize,
        #[clap(long, short = 'u')]
        num_unique_objects: usize,
        #[clap(long, short = 'n')]
        num_requests: usize,
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
            experiment_using_events_path::<Flow>(
                &events_path,
                cache_counts,
                cache_capacity,
                &miss_latency,
            );
        }
        Experiment::StorageTrace {
            events_path,
            cache_counts,
            cache_capacity,
            miss_latency,
        } => {
            experiment_using_events_path::<BlockId>(
                &events_path,
                cache_counts,
                cache_capacity,
                &miss_latency,
            );
        }
        Experiment::IbmKvTrace {
            events_path,
            cache_counts,
            cache_capacity,
            miss_latency,
        } => {
            experiment_using_events_path::<KVObjectId>(
                &events_path,
                cache_counts,
                cache_capacity,
                &miss_latency,
            );
        }
        Experiment::NetworkTraceAnalysis { events_path } => {
            analyze_events::<Flow>(&events_path);
        }
        Experiment::StorageTraceAnalysis { events_path } => {
            analyze_events::<BlockId>(&events_path);
        }
        Experiment::IbmKvTraceAnalysis { events_path } => {
            analyze_events::<KVObjectId>(&events_path);
        }
        Experiment::IrtRatioTest {
            cache_counts,
            cache_capacity,
            miss_latency,
            expected_rearrival_time,
            num_unique_objects,
            num_requests,
        } => {
            let mut rng = XorShiftRng::seed_from_u64(244);
            irt_ratio_experiment::irt_ratio_experiment(
                &mut rng,
                expected_rearrival_time,
                num_unique_objects,
                num_requests,
                cache_counts,
                cache_capacity,
                miss_latency,
            );
        }
    }
}
