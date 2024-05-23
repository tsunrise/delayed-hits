# Replicate Result: Cache with Delayed Hits

This repository contains the code to replicate the result of the paper "Cache with Delayed Hits" by Nirav Atre et al \[[link](https://dl.acm.org/doi/10.1145/3387514.3405883)\].

## Replicate LRU-MAD latency on Network Trace

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
