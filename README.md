# tpc-graph-exec-rs

A "thread per core" architecture implementation in Rust that builds & executes a graph of processing nodes. The design is intended to provide boilerplate for high-performance graph processing architectures with the following features:
* Inter-node/thread communication uses bounded channels for low-latency as well as providing backpressure control. 
* Inter-node SPSC queues use [`rtrb` crate](https://crates.io/crates/rtrb), which is somewhat faster than similar crates like [`ringbuf`](https://github.com/agerasev/ringbuf) (see other performance compares in [this GitHub issue](https://github.com/mgeier/rtrb/issues/39), the performance gap mainly comes from false sharing- ringbuf's head and tail indices can share cache lines, causing unnecessary cache invalidation across cores- and atomic overhead- ringbuf reads atomic indices more frequently than optimal).
* Each node is in an independent, pre-spawned OS thread to minimize spawning overhead. Note this architecture is meant for compute-bound tasks rather than spikey, I/O bound tasks that would be better served by Rust `async`, "green threaded" architectures (such as `tokio`).
  + Each thread is named by the worker `Node` and can be seen via tools like `htop` (`$ htop -u <username>`, then `F2 → Display options → Show custom thread names`).
* Each node can be pinned to a specific CPU core to minimize context switching.
* Per-node performance metrics (polled at 1Hz rate) with generic logging facade to show pipeline status.

To-do:
* [ ] Channels and data structures are cache-optimized to minimize latency of moving between core threads.

> In @src/node.rs we use SPSC `crossbeam_channel` channels to move data between nodes. Since the priority of this crate is low-latency and high-throughput performance, are there ways to improve, or enforce, data layout and alignment of data moving between nodes? For example, would placing `#[repr(align(64))]` or `crossbeam_utils::CachePadded` improve cache efficiency and alignment? Any other obvious places we can improve data movement performance?

```
⏺ Looking at your code, there are several opportunities to optimize data layout and movement for low-latency, high-throughput scenarios. Let me b
reak down the key
  areas:

  1. Channel-Specific Data Alignment

  For data flowing through channels, #[repr(align(64)) can indeed help by:
  - Ensuring each message starts at a cache line boundary
  - Reducing false sharing when producers/consumers access adjacent memory
  - Improving prefetcher efficiency

  // Example user type with cache line alignment
  #[repr(align(64))]
  pub struct AudioBlock {
      samples: [f32; 256],
      timestamp: u64,
  }

  However, this trades memory for performance (padding can waste space), so use it for hot-path types that are frequently sent between cores.

  3. NodeInstance Internal Layout

  Your current NodeInstance struct mixes hot and cold fields. Consider separating them:

⏺ Read(Cargo.toml)
  ⎿  Read 15 lines

⏺ // Optimized layout - separate hot/cold data
  pub struct NodeInstance<I, O> {
      // Hot data - accessed every iteration
      input_rx: Option<Receiver<I>>,
      output_tx: Option<Sender<O>>,
      node: Box<dyn Node<Input = I, Output = O>>,

      // Cold data - accessed once at spawn time
      name: String,
      cpu_core: Option<usize>,

      // Telemetry - could be cache-padded
      bytes_processed_cntr: u64,
  }

  Or use crossbeam_utils::CachePadded for the counter to prevent false sharing with other fields:

  use crossbeam_utils::CachePadded;

  pub struct NodeInstance<I, O> {
      // ...
      bytes_processed_cntr: CachePadded<u64>,
  }

  Though since NodeInstance is moved entirely into its own thread (src/node.rs:130), false sharing is less of a concern here.

  5. Batch Processing

  Instead of processing single items, consider batching for better cache utilization:

  pub trait Node: Send + 'static {
      type Input: Send + 'static;
      type Output: Send + 'static;

      // Process multiple inputs at once
      fn process_batch(&mut self, inputs: Vec<Self::Input>) -> Vec<Self::Output>;
  }

  6. Prefetch Hints

  For predictable access patterns, add prefetch hints:

  #[cfg(target_arch = "x86_64")]
  use std::arch::x86_64::_mm_prefetch;

  // In hot loop, if accessing sequential data
  #[cfg(target_arch = "x86_64")]
  unsafe {
      _mm_prefetch(next_data_ptr as *const i8, _MM_HINT_T0);
  }

  Immediate Actionable Steps

  1. Add #[repr(align(64))] to your most frequently sent data types
  3. Profile with perf to identify actual cache misses:
  perf stat -e cache-misses,cache-references cargo run --release --example simple_graph

  The biggest wins will likely come from choosing the right channel implementation and aligning frequently-sent data types. The internal NodeInst ance layout is less critical since it's thread-local.
```


## Cross-Compilation Targets

### Raspberry Pi 3

1. Add target `$ rustup target add armv7-unknown-linux-musleabihf` (uses `musl` target to statically link `libc` to avoid target version mismatches with ~2-5% performance tradeoff).
2. Make sure Docker is installed and daemon running.
3. Install Docker based [cross-compilation toolchain `cross`](https://github.com/cross-rs/cross) with `$ cargo install cross --git https://github.com/cross-rs/cross`
4. Build with `$ cross build --target armv7-unknown-linux-musleabihf --release`

