use std::{thread, time};

use crossbeam_channel::bounded;
use log::info;
use tpc_graph_exec_rs::node::{Node, NodeInstance};

/// Example node that multiplies input numbers by a given factor
struct MultiplierNode {
    factor: i32,
}

impl Node for MultiplierNode {
    type Input = i32;
    type Output = i32;

    fn process(&mut self, input: Option<Self::Input>) -> Option<Self::Output> {
        Some(input.unwrap() * self.factor)
    }
}

/// Example node that filters even numbers
struct FilterEvenNode;

impl Node for FilterEvenNode {
    type Input = i32;
    type Output = i32;

    fn process(&mut self, input: Option<Self::Input>) -> Option<Self::Output> {
        if input.unwrap() % 2 == 0 {
            Some(input.unwrap())
        } else {
            None
        }
    }
}

/// Example node that prints values
struct PrinterNode {}

impl Node for PrinterNode {
    type Input = i32;
    type Output = ();

    fn process(&mut self, input: Option<Self::Input>) -> Option<Self::Output> {
        info!("Received: {}", input.unwrap());
        None
    }
}

/// Example source node that generates data
struct SourceNode {
    cntr: i32,
}

impl Node for SourceNode {
    type Input = ();
    type Output = i32;

    fn process(&mut self, _input: Option<Self::Input>) -> Option<Self::Output> {
        self.cntr += 1;
        thread::sleep(time::Duration::from_secs(1));
        Some(self.cntr)
    }
}

fn main() {
    env_logger::init();

    let mut source_node = NodeInstance::new("source".to_string(), SourceNode { cntr: 0 });
    let mut print_node = NodeInstance::new("printer".to_string(), PrinterNode {});

    let (tx, rx) = bounded(5);

    source_node.set_sender(tx);
    print_node.set_receiver(rx);

    let source_tdx = source_node.spawn();
    let print_tdx = print_node.spawn();

    source_tdx.unwrap().join().unwrap();
    print_tdx.unwrap().join().unwrap();

    info!("Graph processing completed!");
}
