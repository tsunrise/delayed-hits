use derivative::Derivative;
use rayon::prelude::*;
use serde::de::DeserializeOwned;
use std::fmt::Display;

use clap::{Parser, Subcommand};
use proj_cache_sim::{
    cache::{construct_k_way_cache, lru::LRU, ObjectId},
    simulator::{compute_statistics, run_simulation},
};
use proj_models::{RequestEvent, RequestId};

mod data;

pub trait EventsIterable<K> {
    fn iter_events(&self) -> impl Iterator<Item = RequestEvent> + '_;
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

/// Run an experiment with the given parameters.
/// - `events`: the events to simulate
/// - `cache_counts`: the number of caches in the cache hierarchy
/// - `cache_capacity`: the capacity of each cache
/// - `miss_latency`: the latency of a cache miss
/// - `warmup`: the number of requests to warm up the cache. The warmup requests are not included in the statistics.
fn run_experiment<K, E>(
    events: E,
    cache_counts: usize,
    cache_capacity: usize,
    miss_latency: u64,
    warmup: usize,
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

fn experiment_using_events_path<T>(
    trace_events_path: &[String],
    cache_counts: usize,
    cache_capacity: usize,
    miss_latency: &[u64],
    warmup: usize,
) where
    T: ObjectId + DeserializeOwned + Send + Sync,
{
    let trace_events = EventsLoader::<T>::new(trace_events_path);
    // run_experiment(trace_events, cache_counts, cache_capacity, miss_latency);
    let result = miss_latency
        .par_iter()
        .map(|&miss_latency| {
            run_experiment(
                trace_events,
                cache_counts,
                cache_capacity,
                miss_latency,
                warmup,
            )
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
    let irt = proj_cache_sim::heuristics::median_irt(&trace_events);
    println!("median irt: {}", irt);
}

#[derive(Debug, Subcommand)]
enum Experiment {
    Trace {
        #[clap(long, short = 'p')]
        event_path: String,
        #[clap(long, short = 'k')]
        cache_counts: usize,
        #[clap(long, short = 'c')]
        cache_capacity: usize,
        #[clap(long, short = 'l', help = "miss latency in milliseconds")]
        miss_latency: Vec<u64>,
        #[clap(
            long,
            short = 'w',
            help = "number of warmup requests",
            default_value = "0"
        )]
        warmup: usize,
    },
    Analysis {
        #[clap(long, short = 'p')]
        event_path: String,
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
        Experiment::Trace {
            event_path,
            cache_counts,
            cache_capacity,
            miss_latency,
            warmup,
        } => {
            experiment_using_events_path::<RequestId>(
                &[event_path],
                cache_counts,
                cache_capacity,
                &miss_latency,
                warmup,
            );
        }
        Experiment::Analysis { event_path } => {
            analyze_events::<RequestId>(&[event_path]);
        }
    }
}
