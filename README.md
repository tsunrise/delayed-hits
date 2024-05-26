

# Replicate Result: Cache with Delayed Hits

This repository contains the code to replicate the result of the paper "Cache with Delayed Hits" by Nirav Atre et al \[[link](https://dl.acm.org/doi/10.1145/3387514.3405883)\].

**Significant Refactoring in Progress**

## Setup

1. Install Rust by following the instructions at <https://rustup.rs/>.
2. Get `aria2` from <https://aria2.github.io/>. Make sure `aria2c` is in your path. In ubuntu, you can install it by running

```sh
sudo apt-get install aria2
```

3. Make sure you have `gunzip` installed. 

## Download and Preprocess the Network Trace

We have the following traces available to use:

- `data/net-traces/chicago.toml`: The Chicago trace at 2014-03-20 from 13:50 to 14:00. This trace is used by the authors in the paper.
- `data/net-traces/chicago-lite.toml`: A smaller version of the trace used by the authors in the paper, mainly for testing purposes.

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

## Simulate LRU and LRU-MAD on processed trace

To simulate LRU and LRU-MAD on the processed trace, run the following:

```sh
cargo run --bin proj-experiments --release -- trace -p data/net-traces/<trace_name>/processed.events -c <num_caches> -k <cache_associativity> -l <latency_in_ns> -w <warmup>
```

<!-- ## Replicate LRU-MAD latency on Network Trace

1. Download the pcap network traces from CAIDA: <https://data.caida.org/datasets/passive-2019/equinix-nyc/>. You need to request access.

2. Preprocess the pcap files to `events` file by running

```sh
cargo run --release --bin proj-preprocess -- traces 
    -p <paths_to_pcap_file> 
    -p <additional_path>
    ... 
    -i <path_to_the_first_pcap_file_used_in_experiment>
    --output processed.events
```

### Example
Suppose we want to process `1.pcap`, `2.pcap`, and `3.pcap` files. We can run either run in one command to get a single `processed.events` file:
```sh
cargo run --release --bin proj-preprocess -- traces 
    -p 1.pcap 
    -p 2.pcap 
    -p 3.pcap 
    -i 1.pcap
    --output processed.events
```

or, we can run multiple commands to get multiple `processed.events` files, and simulation can take all of them in order
```sh
cargo run --release --bin proj-preprocess -- traces -p 1.pcap -i 1.pcap --output processed-1.events
cargo run --release --bin proj-preprocess -- traces -p 2.pcap -i 1.pcap --output processed-2.events
cargo run --release --bin proj-preprocess -- traces -p 3.pcap -i 1.pcap --output processed-3.events
```

**Why I need to provide `-i` flag?**

We use `u64` to store the nanosecond timestamp. `u64` is not big enough to store the nanoseconds from the beginning of the year 1970. Instead, we store the nanoseconds from the beginning of the pcap file, which is more than enough. If you provide multiple pcap files, we need to know the starting time of the first pcap file to calculate the timestamp correctly.

3. Run simulation to compare the latency of LRU-MAD with LRU, using the following command

```sh
cargo run --bin proj-experiments --release -- network-trace -p processed.events -k <number-of-caches> -c <cache-capacity> -l <latency>
```

- `number-of-caches` is the parameter `K` for K-way set associative cache.
- `cache-capacity` is the parameter `C` for the cache capacity.
- `latency` is the parameter `L` for the latency of a cache miss, in nanoseconds.

That is, we have `K` caches where each object is distributed to a cache based on the hash of the object. Each cache has a capacity of `C` objects. 

If you have **multiple** `processed.events` files, you can run the following command:

```sh
cargo run --bin proj-experiments --release -- network-trace -p processed-1.events
                                                            -p processed-2.events
                                                            -p processed-3.events
                                                            -k <number-of-caches> -c <cache-capacity> -l <latency>
```

Parallel execution on different latencies are supported. You can run the following command to run the simulation on different latencies. Be mindful of memory usage.

```sh
cargo run --bin proj-experiments --release -- network-trace -p processed-1.events
                                                            -p processed-2.events
                                                            -p processed-3.events
                                                            -k <number-of-caches> -c <cache-capacity> 
                                                            -l <latency-experiment-1>
                                                            -l <latency-experiment-2>
                                                            ...
```

You can get more information about the parameters by running

```sh
cargo run --bin proj-experiments --release -- network-trace --help 
```

You might need to estimate cache capacity using the maximum number of active objects. You can use the following command to get the maximum number of active objects.

```sh
cargo run --bin proj-experiments --release -- network-trace-analysis -p processed-1.events
                                                                     -p processed-2.events
                                                                     ...
```

## Replicate LRU-MAD latency on Storage trace

1. Download the MSN storage file server from <http://iotta.snia.org/traces/block-io/158?n=10&page=2>

2. Preprocess the csv files to `events` file by running

```sh
cargo run --bin proj-preprocess --release -- storage-traces -p data/MSNStorageCFS/Traces/CFS.2008-03-10.01-06.trace.csv.csv -o data/storage-events/MSNFS.meta.events
```

3. Run simulation to compare the latency of LRU-MAD with LRU, using the following command

```sh
cargo run --bin proj-experiments --release -- storage-trace -p data/storage-events/MSNFS.meta.events -k <number-of-caches> -c <cache-capacity> -l <latency-in-microsecond>
```

To analyze the storage trace, you can run the following command

```sh
cargo run --bin proj-experiments --release storage-trace-analysis -p <path-to-storage-trace>
``` -->
