use log::info;
use std::thread;
use tpc_graph_exec_rs::add;
use tpc_graph_exec_rs::{
    graph::Graph,
    node::{Node, NodeInstance},
};

/// Example node that multiplies input numbers by a given factor
struct MultiplierNode {
    factor: i32,
}

impl Node for MultiplierNode {
    type Input = i32;
    type Output = i32;

    fn process(&mut self, input: Self::Input) -> Option<Self::Output> {
        Some(input * self.factor)
    }

    fn on_start(&mut self) {
        info!("MultiplierNode started (factor: {})", self.factor)
    }

    fn on_stop(&mut self) {
        info!("MultiplierNode stopped")
    }
}

/// Example node that filters even numbers
struct FilterEvenNode;

impl Node for FilterEvenNode {
    type Input = i32;
    type Output = i32;

    fn process(&mut self, input: Self::Input) -> Option<Self::Output> {
        if input % 2 == 0 { Some(input) } else { None }
    }

    fn on_start(&mut self) {
        info!("Filter node started");
    }
}

/// Example node that prints values
struct PrinterNode {
    name: String,
}

impl Node for PrinterNode {
    type Input = i32;
    type Output = ();

    fn process(&mut self, input: Self::Input) -> Option<Self::Output> {
        info!("[{}] Received: {}", self.name, input);
        None
    }

    fn on_start(&mut self) {
        info!("[{}] Printer node started", self.name);
    }
}

/// Example source node that generates data
struct SourceNode {
    data: Vec<i32>,
}

impl Node for SourceNode {
    type Input = ();
    type Output = i32;

    fn process(&mut self, _input: Self::Input) -> Option<Self::Output> {
        self.data.pop()
    }

    fn on_start(&mut self) {
        info!("Source node started");
    }
}

fn main() {
    env_logger::init();
    let mut graph = Graph::new();

    // Create nodes
    let (mut source_node, source_tx) = NodeInstance::new("source".to_string(), 100);
    let (mut multiplier1, mult1_tx) = NodeInstance::new("mult1".to_string(), 100);
    let (mut multiplier2, mult2_tx) = NodeInstance::new("mult2".to_string(), 100);
    let (mut filter, filter_tx) = NodeInstance::new("filter".to_string(), 100);
    let (printer1, printer1_tx) = NodeInstance::new("printer1".to_string(), 100);
    let (printer2, printer2_tx) = NodeInstance::new("printer2".to_string(), 100);

    // Connect: source -> (mult1, mult2)
    let mult1_rx = source_node.add_output(100);
    let mult2_rx = source_node.add_output(100);

    // Connect: mult1 -> filter -> printer1
    let filter_rx = multiplier1.add_output(100);
    let printer1_rx = filter.add_output(100);

    // Connect: mult2 -> printer2
    let printer2_rx = multiplier2.add_output(100);

    // Spawn all nodes
    graph.add_handle(source_node.spawn(SourceNode {
        data: (1..=10).collect(),
    }));

    graph.add_handle(multiplier1.spawn(MultiplierNode { factor: 2 }));
    graph.add_handle(multiplier2.spawn(MultiplierNode { factor: 3 }));
    graph.add_handle(filter.spawn(FilterEvenNode));

    graph.add_handle(printer1.spawn(PrinterNode {
        name: "Mult2-Filter".to_string(),
    }));

    graph.add_handle(printer2.spawn(PrinterNode {
        name: "Mult3".to_string(),
    }));

    // Set up data flow threads
    let h1 = thread::spawn(move || {
        while let Ok(val) = mult1_rx.recv() {
            if mult1_tx.send(val).is_err() {
                break;
            }
        }
    });

    let h2 = thread::spawn(move || {
        while let Ok(val) = mult2_rx.recv() {
            if mult2_tx.send(val).is_err() {
                break;
            }
        }
    });

    let h3 = thread::spawn(move || {
        while let Ok(val) = filter_rx.recv() {
            if filter_tx.send(val).is_err() {
                break;
            }
        }
    });

    let h4 = thread::spawn(move || {
        while let Ok(val) = printer1_rx.recv() {
            if printer1_tx.send(val).is_err() {
                break;
            }
        }
    });

    let h5 = thread::spawn(move || {
        while let Ok(val) = printer2_rx.recv() {
            if printer2_tx.send(val).is_err() {
                break;
            }
        }
    });

    // Send data to source node
    for _ in 0..10 {
        source_tx.send(()).unwrap();
    }
    drop(source_tx);

    // Wait for all threads
    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();
    h5.join().unwrap();

    graph.wait();

    info!("\nGraph processing completed!");

    assert_eq!(add(1, 3), 4);
}
