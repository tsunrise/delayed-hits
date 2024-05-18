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

3. Run simulation to compare the latency of LRU-MAD with LRU, using the following command

```sh
cargo run --bin proj-experiments -- network-trace -p processed.events -k <number-of-caches> -c <cache-capacity> -l <latency>
```

You can get more information about the parameters by running

```sh
cargo run --bin proj-experiments -- network-trace --help
```
