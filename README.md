# Replicate Result: Cache with Delayed Hits

This repository contains the code to replicate the result of the paper "Cache with Delayed Hits" by Nirav Atre et al \[[link](https://dl.acm.org/doi/10.1145/3387514.3405883)\].

## Replicate LRU-MAD latency on Network Trace

1. Download the pcap network traces from CAIDA: <https://data.caida.org/datasets/passive-2019/equinix-nyc/>. You need to request access.

2. Preprocess the pcap files to `events` file by running

```sh
cargo run --bin proj-preprocess -- 
    --ftype traces 
    --paths <paths_to_pcap_file> 
    --paths <additional_path>
    ... 
    --output processed.events
```

If you need to concat Net Events file:
```sh
cargo run --bin proj-preprocess -- 
    --ftype processed-net-events
    --paths <paths_to_events> 
    --paths <additional_path>
    ... 
    --output processed.events
```

3. Run simulation to compare the latency of LRU-MAD with LRU, using the following command

```sh
cargo run --bin proj-experiments --release -- network-trace -p processed.events -k <number-of-caches> -c <cache-capacity> -l <latency>
```

- `number-of-caches` is the parameter `K` for K-way set associative cache.
- `cache-capacity` is the parameter `C` for the cache capacity.
- `latency` is the parameter `L` for the latency of a cache miss, in nanoseconds.

That is, we have `K` caches where each object is distributed to a cache based on the hash of the object. Each cache has a capacity of `C` objects. 

You can get more information about the parameters by running

```sh
cargo run --bin proj-experiments --release -- network-trace --help
```

You might need to estimate cache capacity using the maximum number of active objects. You can use the following command to get the maximum number of active objects.

```sh
cargo run --bin proj-experiments --release -- network-trace-analysis -p <processed-events>
```
