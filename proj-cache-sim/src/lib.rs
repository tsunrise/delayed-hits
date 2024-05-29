pub mod cache;
pub mod heuristics;
pub mod io;
pub mod macros;
pub mod simulator;
pub mod types;

pub fn parse_time_unit(s: &str) -> Result<u64, std::num::ParseIntError> {
    match s {
        s if s.ends_with("ns") => s[..s.len() - 2].parse(),
        s if s.ends_with("us") => Ok(s[..s.len() - 2].parse::<u64>()? * 1000),
        s if s.ends_with("ms") => Ok(s[..s.len() - 2].parse::<u64>()? * 1_000_000),
        s if s.ends_with("s") => Ok(s[..s.len() - 1].parse::<u64>()? * 1_000_000_000),
        _ => s.parse(),
    }
}

pub fn get_time_string(nanos: u128) -> String {
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
