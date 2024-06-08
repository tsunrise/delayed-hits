# Replicate "Caching with Delayed Hits"

This repository contains the code to replicate the result of the paper "Cache with Delayed Hits" by Nirav Atre et al \[[link](https://dl.acm.org/doi/10.1145/3387514.3405883)\].

Check our blog post: https://reproducingnetworkresearch.wordpress.com/?p=13178

## Setup

1. Install Rust by following the instructions at <https://rustup.rs/>. The code is tested with rustc 1.78.0 (9b00956e5 2024-04-29) and is platform-independent. 

    Note: If the code does not compile in future versions of Rust, it's highly likely that the `npy` crate used by `proj-toy-cdn` is not compatible. You can try to remove the `npy` dependency: we use this dependency to generate `.npy` files for plotting the results. You can safely remove the corresponding code without affecting the core functionality.

2. To download the network trace using the script, make sure at least one of `aria2c`, `wget`, `curl` is in your path. In ubuntu, you can install `aria2c` by running

```sh
sudo apt-get install aria2
```

Also, Make sure you have `gunzip` installed.

## Preprocess Trace

As the first step, we need to convert raw traces of different format to a common binary format, stored in a `.events` file. The events file contains a stream of tuples `(key: u64, timestamp: u64)` encoded in little-endian format.

### Download and Preprocess the Network Trace

We have the following trace available to use:

- `data/net-traces/chicago.toml`: The Chicago trace at 2014-03-20 from 13:50 to 14:00. This trace is used by the authors in the paper.
-  You can adapt the toml file to download other traces from CAIDA.

Here is how to download and preprocess the Chicago trace:

1. Request dataset access from CAIDA: <https://www.caida.org/catalog/datasets/request_user_info_forms/passive_dataset_request/>. You need have access to

    - CAIDA Anonymized Internet Traces 2014 Dataset (high-speed traces/commercial backbone)
2. Download the preprocessed network trace from CAIDA by running the following command:

```sh
python3 scripts/download_traces.py <path_to_toml_file> --user <username> --password <password>
```

The username and password are the ones you used to request access to the dataset.

3. Preprocess the downloaded trace by running:

```sh
cargo run --bin proj-preprocess --release -- net-traces <path_to_toml_file>
```
The processed file will be saved to `data/net-traces/<name>/processed.events` for `<name>.toml`.

### Download the Storage Trace

You can follow the README in [data/ms_prod/README.md](data/ms_prod/README.md) to download the raw traces. Then, preprocess the trace by running:
```sh
cargo run --release --bin proj-preprocess -- ms-prod-traces
```

### Download the processed CDN Trace

The raw CDN trace is not available for download. You can download the processed trace [here](https://r2.tomshen.io/proj_host/cs244/cdn_long.downloaded.events.gz). After downloading, you can unzip the file using `gunzip`.
To verify the integrity of the unzipped file, the SHA1 hash is [here](data/cdn-traces/cdn_long.downloaded.events.sha1).

## Simulate LRU and LRU-MAD on processed trace

To simulate LRU and LRU-MAD on the processed trace, run the following:

```sh
cargo run --bin proj-experiments --release -- trace -p <trace_name>.events
    -k <num_caches>
    -c <num_lines_in_each_cache> 
    -l <latency> 
    -w <warmup>
```

For latency, you can use `ms` for milliseconds, `us` for microseconds, and `ns` for nanoseconds. If you do not provide the unit, it will be assumed to be nanoseconds.

For example, to simulate a 64-way 128-set associative cache with 30ms load latency and 5000000 warmup requests on the Chicago-lite trace, run:

```sh
cargo run --bin proj-experiments --release -- trace -p data/net-traces/chicago-lite/processed.events -c 128 -k 64 -l 30ms -w 5000000
```

## CDN Emulation Experiment

First, you need to have access to two hosts at geographically distinct locations. Make sure at least one host has a public IP address. The two hosts, `cdn` and `origin` are connected via multiple long-run TCP connections. One host listens on a port and the other connects to it. Make sure to run the listener first.

**Origin**: To start the origin server, do all the environment setup and run the following command:

```sh
cargo run --release --bin proj-toy-origin -- -c <addr> -n <num_tcp> -b <buffer_size>
```

- `addr`: If origin is listening to a port (let's say 12244), use that port number (e.g. `12244`). If origin is connecting to a port, use the IP address and port number (let's say `1.2.3.4` and port `12244`), use `<IP>:<port>` (e.g. `1.2.3.4:12244`).
- `num_tcp`: Number of TCP connections to establish with the CDN.
- `buffer_size`: TCP read/write buffer size, in terms of number of messages. Each request message takes 8 bytes, so the actual read buffer size is `buffer_size * 8` bytes. Each response message takes 16 bytes, so the actual write buffer size is `buffer_size * 16` bytes.

**CDN**: To start the CDN server, do all the environment setup and run the following command:

```sh
cargo run --release --bin proj-toy-cdn -- 
    -c <addr> 
    -n <num_tcp>
    -b <buffer_size>
    experiment <path_to_cdn_trace.events>
    -k <num_caches>
    -c <num_lines_in_each_cache>
    -w <warmup>
    -m <num_requests_after_warmup>
    -t <type_of_cache>
    -l <latency>
    -i <irt>
```

- `addr`: `<port>` for listening on a port, `<IP>:<port>` for connecting to a port.
- `num_tcp`: Number of TCP connections to establish with the origin.
- `buffer_size`: TCP read/write buffer size, in terms of number of messages. Note that each request message takes 16 bytes, so the actual write buffer size is `buffer_size * 16` bytes. Each response message takes 8 bytes, so the actual read buffer size is `buffer_size * 8` bytes.
- `path_to_cdn_trace.events`: Path to the CDN trace.
- `num_caches`: Number of caches in the CDN.
- `num_lines_in_each_cache`: Number of lines in each cache.
- `warmup`: Number of warmup requests. Refer to the blog post for more details.
- `num_requests_after_warmup`: Number of requests after warmup.
- `type_of_cache`: Type of cache to use. Can be `lru` or `lru-mad`.
- `latency`: Latency of the cache for the simulation during warmup, in terms of milliseconds, microseconds, or nanoseconds. If you do not provide the unit, it will be assumed to be nanoseconds.
- `irt`: Inter-request interval, in terms of milliseconds, microseconds, or nanoseconds. If you do not provide the unit, it will be assumed to be nanoseconds. (default: 1us)

For example, to run the CDN emulation experiment on the CDN trace with 128-way 512-set associative cache with 5ms latency, 1000000 warmup requests, and 500000 actual requests after warmup, using LRU cache, and 3us inter-request interval, run:

```sh
cargo run --release --bin proj-toy-cdn -- 
    -c 12244 
    -n 12 
    -b 4 
    experiment data/cdn-traces/cdn_long.processed.events 
    -k 512 -c 128 -w 1000000 -m 500000 -t lru -l 5ms -i 3us
```

#### Test RTT under high load

To test the round-trip time (RTT) under high load, you can use the same origin setup. For the CDN, run the following command:

```sh
 cargo run --release --bin proj-toy-cdn -- 
    -c <addr> 
    -n <num_tcp>
    -b <buffer_size>
    bench -r <num_requests>
```

The arguments are the same as the previous command, except for the `bench` subcommand. `num_requests` is the number of requests to send to the origin. Those requests are sent every 1us.
