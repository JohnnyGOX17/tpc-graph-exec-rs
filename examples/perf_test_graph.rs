use log::info;
use std::env::set_var;
use tpc_graph_exec_rs::connect_nodes;
use tpc_graph_exec_rs::node::{Node, NodeInstance};

// use faster/smaller `mimalloc` allocator
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// Example node that multiplies input numbers by a given factor
struct MultiplierNode {
    factor: i32,
}

impl Node for MultiplierNode {
    type Input = Vec<i32>;
    type Output = Vec<i32>;

    fn process(&mut self, input: Option<Self::Input>) -> Option<Self::Output> {
        let mut output = vec![0; input.unwrap().len()];
        for i in output.iter_mut() {
            *i *= self.factor;
        }
        Some(output)
    }
}

/// Example node that prints values
struct SinkNode {}

impl Node for SinkNode {
    type Input = Vec<i32>;
    type Output = ();

    fn process(&mut self, _input: Option<Self::Input>) -> Option<Self::Output> {
        None
    }
}

struct SourceNode {}

impl Node for SourceNode {
    type Input = ();
    type Output = Vec<i32>;

    fn process(&mut self, _input: Option<Self::Input>) -> Option<Self::Output> {
        Some((0..4096).collect())
    }
}

fn main() {
    unsafe { set_var("RUST_LOG", "INFO") };
    env_logger::init();

    let mut source_node = NodeInstance::new("source".to_string(), SourceNode {}, Some(6));
    let mut mult_node =
        NodeInstance::new("mult x3".to_string(), MultiplierNode { factor: 3 }, Some(7));
    let mut sink_node = NodeInstance::new("sink".to_string(), SinkNode {}, Some(8));

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
