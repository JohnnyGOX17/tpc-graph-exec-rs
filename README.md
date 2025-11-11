# tpc-graph-exec-rs

A "thread per core" architecture implementation in Rust that builds & executes a graph of processing nodes. The design is intended to provide boilerplate for high-performance graph processing architectures with the following features:
* Inter-node/thread communication uses bounded channels for low-latency as well as providing backpressure control. 
* [ ] Channels and data structures are cache-optimized to minimize latency of moving between core threads.
* Each node is in an independent, pre-spawned OS thread to minimize spawning overhead. Note this architecture is meant for compute-bound tasks rather than spikey, I/O bound tasks that would be better served by Rust `async`, "green threaded" architectures (such as `tokio`).
* [ ] Each node can be pinned to a specific CPU core to minimize context switching.


