# tpc-graph-exec-rs

A "thread per core" architecture implementation in Rust that builds & executes a graph of processing nodes. The design is intended to provide boilerplate for high-performance graph processing architectures with the following features:
* Inter-node/thread communication uses bounded channels for low-latency as well as providing backpressure control. 
* Each node is in an independent, pre-spawned OS thread to minimize spawning overhead. Note this architecture is meant for compute-bound tasks rather than spikey, I/O bound tasks that would be better served by Rust `async`, "green threaded" architectures (such as `tokio`).
  + Each thread is named and can be seen in `$ htop -u <username>` and then `F2 → Display options → Show custom thread names`.
* Each node can be pinned to a specific CPU core to minimize context switching.

To-do:
* [ ] Channels and data structures are cache-optimized to minimize latency of moving between core threads.
* [ ] Per-node performance metrics and generic logging at configurable rate

## Cross-Compilation Targets

### Raspberry Pi 3

1. Add target `$ rustup target add armv7-unknown-linux-musleabihf` (uses `musl` target to statically link `libc` to avoid target version mismatches with ~2-5% performance tradeoff).
2. Make sure Docker is installed and daemon running.
3. Install Docker based [cross-compilation toolchain `cross`](https://github.com/cross-rs/cross) with `$ cargo install cross --git https://github.com/cross-rs/cross`
4. Build with `$ cross build --target armv7-unknown-linux-musleabihf --release`

