use log::info;
use tpc_graph_exec_rs::connect_nodes;
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

/// Example node that prints values
struct SinkNode {}

impl Node for SinkNode {
    type Input = i32;
    type Output = ();

    fn process(&mut self, _input: Option<Self::Input>) -> Option<Self::Output> {
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
        Some(self.cntr)
    }
}

fn main() {
    env_logger::init();

    let mut source_node = NodeInstance::new("source".to_string(), SourceNode { cntr: 0 }, Some(6));
    let mut mult_node =
        NodeInstance::new("mult x3".to_string(), MultiplierNode { factor: 3 }, Some(7));
    let mut sink_node = NodeInstance::new("printer".to_string(), SinkNode {}, Some(8));

    connect_nodes!(source_node -> mult_node, 16);
    connect_nodes!(mult_node -> sink_node, 16);

    let threads = vec![
        source_node.spawn().unwrap(),
        mult_node.spawn().unwrap(),
        sink_node.spawn().unwrap(),
    ];

    for tdx in threads {
        tdx.join().unwrap();
    }

    info!("Graph processing completed!");
}
