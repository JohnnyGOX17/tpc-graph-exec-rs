# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**tpc-graph-exec-rs** is a "thread per core" graph processing framework in Rust designed for high-performance, compute-bound workloads. It provides a foundation for building systems where:
- Processing nodes run in pre-spawned OS threads (one per node)
- Inter-node communication uses bounded channels for low-latency message passing and backpressure control
- Each node independently processes input, transforming it, and optionally producing output

## Architecture

### Core Components

1. **Node trait** (`src/node.rs:4`): The fundamental abstraction that defines a processing unit
   - `Input` and `Output` associated types (must be `Send + 'static`)
   - `process(&mut self, input: Input) -> Option<Output>`: Main processing method; returns `None` to drop data
   - `on_start()` and `on_stop()`: Lifecycle hooks for initialization and cleanup

2. **NodeInstance** (`src/node.rs:32`): Generic wrapper for a node that manages its channels
   - Holds a bounded input channel receiver and multiple output channel senders
   - Spawns the node in its own thread via `spawn<N>()`
   - All connected outputs receive cloned copies of the node's output data

3. **Graph** (`src/graph.rs:4`): High-level container for managing node lifecycle
   - Collects `NodeHandle`s from spawned nodes
   - `wait()` method blocks until all nodes complete (joins all threads)

### Data Flow Pattern

```
Source Node -> (output channels) -> Downstream Node 1 -> Output
                                 -> Downstream Node 2 -> Output
```

- Each node can have multiple output consumers (via `add_output()`)
- Output data is cloned to all downstream consumers
- A node stops when its input channel is closed (sender dropped or all senders gone)
- If a node's `process()` returns `None`, the data is discarded and not sent downstream

## Common Development Tasks

### Building the Project

```bash
cargo build              # Debug build
cargo build --release   # Optimized release build
```

### Running Tests

```bash
cargo test              # Run all tests
cargo test -- --nocapture  # Show println!/log output
cargo test --lib        # Run library tests only
```

### Running Examples

```bash
cargo run --example simple_graph                          # Basic example with logging
RUST_LOG=info cargo run --example simple_graph           # Show info-level logs
RUST_LOG=debug cargo run --example simple_graph          # Show debug-level logs
```

The example demonstrates:
- Creating nodes that implement the `Node` trait
- Connecting nodes via channels
- Setting up data flow between processing stages
- Spawning threads and managing their lifecycle

### Linting & Formatting

```bash
cargo clippy            # Run linter with suggestions
cargo fmt              # Format code
cargo fmt -- --check   # Check formatting without changes
```

## Key Design Considerations

### Thread Safety

- All data flowing through channels must implement `Send + 'static`
- Node implementations can safely mutate their local state (no `Arc<Mutex<>>` needed for per-node state)
- The framework does not provide thread-safe global state; use exterior mutability patterns if needed

### Channel Backpressure

- All channels are bounded SPSC queues
- When a node's input buffer is full, upstream senders will block until space is available
- Buffer sizes are specified when creating `NodeInstance` (see `examples/simple_graph.rs` for typical values like 100)

### Output Broadcasting

- A single node can send output to multiple downstream nodes via `add_output()`
- Output data is cloned when sent to multiple consumers
- This pattern is visible in `simple_graph.rs` where the source node feeds into two different multiplier nodes

### Graceful Shutdown

- Close the input sender to a node to trigger shutdown
- Nodes will drain remaining queued inputs and call `on_stop()` before thread termination
- Use `Graph::wait()` to block until all threads finish

## Testing Notes

- The example in `src/lib.rs` contains a placeholder test (`it_works`)
- New functionality should be tested through the `#[cfg(test)]` module pattern
- For integration tests, consider using `examples/` as a template for full graph scenarios
