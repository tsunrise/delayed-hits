mod data;
use clap::{Parser, Subcommand};
use proj_cache_sim::heuristics;

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
        #[clap(required = true)]
        event_path: String,
    },
}

fn analyze_event(event_path: &str) {
    let requests = data::load_data(event_path).collect::<Vec<_>>();
    let maximum_active_objects = heuristics::maximum_active_objects(&requests);
    let mean_irt = heuristics::mean_irt(&requests);

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
            todo!()
        }
        Experiment::Analysis { event_path } => {
            analyze_event(&event_path);
        }
    }
}
