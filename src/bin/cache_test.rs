//! Cache and Memory Latency Measurement via Pointer Chasing
//!
//! Uses a pseudo-random traversal pattern to defeat hardware prefetchers.
//! The stride pattern is based on an LCG permutation to ensure each element
//! is visited exactly once while remaining unpredictable.

use std::hint::black_box;
use std::time::Instant;
use tpc_graph_exec_rs::format_size_fixed_int;

/// Number of iterations to amortize timing overhead
const ITERATIONS: usize = 1_000_000;

/// Generate a permutation of indices using an LCG-based shuffle
/// This defeats spatial and stride prefetchers while guaranteeing full coverage
fn generate_pointer_chase(size: usize) -> Vec<usize> {
    let n = size / std::mem::size_of::<usize>();
    let mut chain = vec![0usize; n];

    // Build the chase: each element points to the next in the permutation
    let mut indices: Vec<usize> = (0..n).collect();

    // Fisher-Yates shuffle with deterministic seed for reproducibility
    let mut rng_state = 0xDEADBEEFu64;
    for i in (1..n).rev() {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let j = (rng_state as usize) % (i + 1);
        indices.swap(i, j);
    }

    // Create circular linked list through the shuffled indices
    for i in 0..n {
        let current = indices[i];
        let next = indices[(i + 1) % n];
        chain[current] = next;
    }

    chain
}

/// Measure latency by chasing pointers through the buffer
#[inline(never)]
fn measure_latency(chain: &[usize], iterations: usize) -> f64 {
    let ptr = chain.as_ptr();
    let mut idx = 0usize;

    // Warmup: traverse once to ensure TLB is populated
    for _ in 0..chain.len() {
        idx = unsafe { *ptr.add(idx) };
    }

    // Timed pointer chase
    let start = Instant::now();

    for _ in 0..iterations {
        // Unroll 8x to reduce loop overhead relative to memory latency
        idx = unsafe { *ptr.add(idx) };
        idx = unsafe { *ptr.add(idx) };
        idx = unsafe { *ptr.add(idx) };
        idx = unsafe { *ptr.add(idx) };
        idx = unsafe { *ptr.add(idx) };
        idx = unsafe { *ptr.add(idx) };
        idx = unsafe { *ptr.add(idx) };
        idx = unsafe { *ptr.add(idx) };
    }

    let elapsed = start.elapsed();

    // Prevent dead code elimination
    black_box(idx);

    let total_accesses = iterations * 8;
    elapsed.as_nanos() as f64 / total_accesses as f64
}

fn main() {
    println!("Cache & Memory Latency Measurement");
    println!("===================================");
    println!("Method: Pointer chasing with randomized permutation");
    println!(
        "Iterations per size: {} ({} accesses with 8x unroll)\n",
        ITERATIONS,
        ITERATIONS * 8
    );

    // Test sizes: 4 KiB to 256 MiB (powers of 2)
    let sizes: Vec<usize> = (12..=28) // 2^12 = 4 KiB to 2^28 = 256 MiB
        .map(|exp| 1usize << exp)
        .collect();

    println!("{:>10} {:>12} {:>10}", "Size", "Latency (ns)", "Est. Level");
    println!("{:-<10} {:-<12} {:-<10}", "", "", "");

    let mut prev_latency = 0.0f64;

    for size in sizes {
        // Generate randomized pointer chase pattern
        let chain = generate_pointer_chase(size);

        // Run measurement multiple times and take median
        let mut samples: Vec<f64> = (0..5)
            .map(|_| measure_latency(&chain, ITERATIONS))
            .collect();
        samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let latency = samples[2]; // Median

        // Heuristic cache level detection based on latency jumps
        let level = if latency < 2.0 {
            "L1d"
        } else if latency < 6.0 {
            "L1d/L2"
        } else if latency < 15.0 {
            "L2"
        } else if latency < 50.0 {
            "L3"
        } else {
            "DRAM"
        };

        // Show jump indicator for significant latency increases
        let jump = if prev_latency > 0.0 && latency > prev_latency * 1.5 {
            " ←"
        } else {
            ""
        };

        println!(
            "{:>10} {:>10.2} ns {:>10}{}",
            format_size_fixed_int(size as u64),
            latency,
            level,
            jump
        );

        prev_latency = latency;
    }

    println!("\n← indicates significant latency jump (cache level transition)");
    println!("\nNote: Actual cache sizes vary by CPU. Typical modern desktop:");
    println!("  L1d: 32-48 KiB, L2: 256 KiB-1 MiB, L3: 8-32 MiB");
}
