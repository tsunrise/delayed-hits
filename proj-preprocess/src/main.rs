use clap::{Parser, ValueEnum};
use proj_models::RequestEvent;

mod example_parser;

fn read_example_events(path: &str) -> Vec<RequestEvent<u32>> {
    let file = std::fs::File::open(path).unwrap();
    example_parser::read_events(file)
}

fn read_traces(path: &str) -> Vec<RequestEvent<u32>> {
    todo!()
}

#[derive(ValueEnum, Debug, Clone)]
enum RunType {
    Example,
    Traces,
}

#[derive(Parser, Debug)]
#[command(about = "Preprocess the example or traces.")]
struct Args {
    #[arg(value_enum, short, long)]
    ftype: RunType,
    #[arg(short, long)]
    path: String,
}

fn main() {
    // <binary> --ftype example --path path/to/example
    // <binary> --ftype traces --path path/to/traces

    let args = Args::parse();
    match args.ftype {
        RunType::Example => {
            let events = read_example_events(&args.path);
            println!("{:?}", events);
        }
        RunType::Traces => {
            let events = read_traces(&args.path);
            println!("{:?}", events);
        }
    }
}
