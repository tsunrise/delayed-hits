use std::time::Duration;

use crate::models::RawRequestWithTimestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MsrBlockIoRequest {
    pub offset: u64,
    pub disk_num: u32,
}

fn parse_line(line: &str) -> Option<RawRequestWithTimestamp<MsrBlockIoRequest>> {
    let mut it = line.split(",");
    let timestamp = it.next()?.parse::<u64>().ok()?; // timestamp in windows filetime (100ns for each timestep, since 1601-01-01)
                                                     // https://stackoverflow.com/questions/6161776/convert-windows-filetime-to-second-in-unix-linux
    const WINDOWS_TICK: u64 = 10000000; // 1s = 10^7 * 100ns
    const SEC_TO_UNIX_EPOCH: u64 = 11644473600; // seconds between 1601-01-01 and 1970-01-01
    let second = timestamp / WINDOWS_TICK - SEC_TO_UNIX_EPOCH;
    let nsec = (timestamp % WINDOWS_TICK) * 100; // 100ns to ns
    let timestamp = Duration::new(second, nsec as u32);
    let _host_name = it.next()?; // host name
    let disk_num = it.next()?.parse::<u32>().ok()?;
    let _op = it.next()?; // operation
    let offset = it.next()?.parse::<u64>().ok()?;
    let _size = it.next()?;
    let _latency = it.next()?;
    debug_assert!(it.next().is_none());
    Some(RawRequestWithTimestamp {
        request: MsrBlockIoRequest { offset, disk_num },
        timestamp,
    })
}

pub fn read_msr_cambridge_requests<R: std::io::BufRead>(
    reader: R,
) -> impl Iterator<Item = RawRequestWithTimestamp<MsrBlockIoRequest>> {
    reader
        .lines()
        .map(|line| parse_line(&line.unwrap()))
        .filter_map(|x| x)
}
