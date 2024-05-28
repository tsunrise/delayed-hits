mod data;
use std::fmt::Display;

use clap::{Parser, Subcommand};
use proj_cache_sim::{
    cache::{construct_k_way_cache, lru::LRU, lru_mad::LRUMinAD},
    heuristics,
    simulator::{compute_statistics, run_simulation},
};

fn get_time_string(nanos: u128) -> String {
    let micros = nanos / 1000;
    let millis = micros / 1000;
    let seconds = millis / 1000;
    if seconds > 0 {
        format!("{} s", seconds)
    } else if millis > 0 {
        format!("{} ms", millis)
    } else if micros > 0 {
        format!("{} us", micros)
    } else {
        format!("{} ns", nanos)
    }
}

fn print_irt_stats(irt_stat: &heuristics::IrtStatistics) {
    println!(
        "Mean inter-request time: {}",
        get_time_string(irt_stat.mean() as u128)
    );
    println!("Inter-request time distribution:");
    for i in 0..10 {
        let ns = 10u64.pow(i as u32);
        let count = irt_stat.buckets[i];
        if count == 0 {
            continue;
        }
        println!(
            "  {} <= irt < {}: {} ({:.2}%)",
            get_time_string(ns as u128),
            get_time_string(10 * ns as u128),
            count,
            count as f64 / irt_stat.count as f64 * 100.0
        );
    }
}

fn analyze_event(event_path: &str) {
    let requests = data::load_data(event_path).collect::<Vec<_>>();
    let maximum_active_objects = heuristics::maximum_active_objects(&requests);
    let irt_stat = heuristics::get_irt(&requests);

    println!("Number of requests: {}", requests.len());
    println!("Maximum active objects: {}", maximum_active_objects);
    print_irt_stats(&irt_stat);

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
    num_loads_lru: usize,
    num_loads_lru_mad: usize,
    improvement: f64,
    cache_counts: usize,
    cache_capacity: usize,
    warmup: usize,
    miss_latency: u64,
}

impl Display for ExperimentResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "k: {}, c: {}, miss_latency: {}\n\
            total latency (lru): {}\n\
            average latency (lru): {}\n\
            num loads (lru): {}\n\
            total latency (lru-mad): {}\n\
            average latency (lru-mad): {}\n\
            num loads (lru-mad): {}\n\
            improvement (%): {}\n\
            CSV: {}, {}, {}, {}, {}, {}",
            self.cache_counts,
            self.cache_capacity,
            self.miss_latency,
            self.total_latency_lru,
            self.average_latency_lru,
            self.num_loads_lru,
            self.total_latency_lru_mad,
            self.average_latency_lru_mad,
            self.num_loads_lru_mad,
            self.improvement * 100.0,
            self.cache_counts,
            self.cache_capacity,
            self.miss_latency,
            self.warmup,
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
    max_requests: Option<usize>,
) -> ExperimentResult {
    let mut lru = construct_k_way_cache(cache_counts, |_| LRU::new(cache_capacity));
    let request_results_lru = if let Some(max_requests) = max_requests {
        // // uncomment this block to simulate the toy cdn deployment (after dummy warmup, the CDN nodes waits for all requests to be fulfilled before playing the trace)
        // let mut requests = data::load_data(requests_path).take(max_requests);
        // let requests_a = requests
        //     .by_ref()
        //     .take(warmup)
        //     .collect::<Vec<_>>()
        //     .into_iter();
        // let requests_b = requests.map(|mut req| {
        //     req.timestamp += miss_latency;
        //     req
        // });
        // run_simulation(&mut lru, requests_a.chain(requests_b), miss_latency)
        run_simulation(
            &mut lru,
            data::load_data(requests_path).take(max_requests),
            miss_latency,
        )
    } else {
        run_simulation(&mut lru, data::load_data(requests_path), miss_latency)
    };

    let stats = compute_statistics(&request_results_lru.results[warmup..]);
    let lru_avg_latency = stats.average_latency;

    let mut lru_mad = construct_k_way_cache(cache_counts, |_| {
        LRUMinAD::new(cache_capacity, miss_latency)
    });
    let request_results_lru_mad = if let Some(max_requests) = max_requests {
        // // uncomment this block to simulate the toy cdn deployment (after dummy warmup, the CDN nodes waits for all requests to be fulfilled before playing the trace)
        // let mut requests = data::load_data(requests_path).take(max_requests);
        // let requests_a = requests
        //     .by_ref()
        //     .take(warmup)
        //     .collect::<Vec<_>>()
        //     .into_iter();
        // let requests_b = requests.map(|mut req| {
        //     req.timestamp += miss_latency;
        //     req
        // });
        // run_simulation(&mut lru_mad, requests_a.chain(requests_b), miss_latency)
        run_simulation(
            &mut lru_mad,
            data::load_data(requests_path).take(max_requests),
            miss_latency,
        )
    } else {
        run_simulation(&mut lru_mad, data::load_data(requests_path), miss_latency)
    };

    let stats = compute_statistics(&request_results_lru_mad.results[warmup..]);
    let lru_mad_avg_latency = stats.average_latency;

    let improvement = (lru_avg_latency - lru_mad_avg_latency) / lru_avg_latency;

    ExperimentResult {
        total_latency_lru: stats.total_latency,
        average_latency_lru: lru_avg_latency,
        num_loads_lru: request_results_lru.num_of_loads,
        total_latency_lru_mad: stats.total_latency,
        average_latency_lru_mad: lru_mad_avg_latency,
        num_loads_lru_mad: request_results_lru_mad.num_of_loads,
        improvement,
        cache_counts,
        cache_capacity,
        warmup,
        miss_latency,
    }
}

fn parse_miss_latency(s: &str) -> Result<u64, std::num::ParseIntError> {
    match s {
        s if s.ends_with("ns") => s[..s.len() - 2].parse(),
        s if s.ends_with("us") => Ok(s[..s.len() - 2].parse::<u64>()? * 1000),
        s if s.ends_with("ms") => Ok(s[..s.len() - 2].parse::<u64>()? * 1_000_000),
        s if s.ends_with("s") => Ok(s[..s.len() - 1].parse::<u64>()? * 1_000_000_000),
        _ => s.parse(),
    }
}

fn head(path: &str, n: usize) {
    let requests = data::load_data(path).take(n);
    for request in requests {
        println!("{}:{}", request.timestamp, request.key)
    }
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
        #[clap(long, short = 'l', help = "miss latency with unit (e.g. 300ns, 2ms)", value_parser = parse_miss_latency)]
        miss_latency: u64,
        #[clap(
            long,
            short = 'w',
            help = "number of warmup requests",
            default_value = "0"
        )]
        warmup: usize,
        #[clap(long, short = 'm', help = "maximum number of requests to process")]
        max_requests: Option<usize>,
    },
    Analysis {
        #[clap(required = true)]
        event_path: String,
    },
    Head {
        #[clap(required = true)]
        event_path: String,
        #[clap(long, short = 'n', default_value = "10")]
        n: usize,
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
            max_requests,
        } => {
            let result = run_experiment(
                &event_path,
                cache_counts,
                cache_capacity,
                miss_latency,
                warmup,
                max_requests,
            );
            println!("{}", result);
        }
        Experiment::Analysis { event_path } => {
            analyze_event(&event_path);
        }
        Experiment::Head { event_path, n } => {
            head(&event_path, n);
        }
    }
}
