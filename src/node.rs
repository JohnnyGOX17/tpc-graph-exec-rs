use crossbeam_channel::{Receiver, Sender, bounded};
use std::thread;

pub trait Node: Send + 'static {
    type Input: Send + 'static;
    type Output: Send + 'static;

    /// Process a single input and produce an output
    fn process(&mut self, input: Self::Input) -> Option<Self::Output>;

    /// Called once when the node starts
    fn on_start(&mut self) {}

    /// Called once when the node stops
    fn on_stop(&mut self) {}
}

/// Handle for a running node
pub struct NodeHandle {
    thread: Option<thread::JoinHandle<()>>,
}

impl NodeHandle {
    pub fn join(mut self) {
        if let Some(thread) = self.thread.take() {
            thread.join().expect("Node thread panicked");
        }
    }
}

/// A node instance in the graph with its channels
pub struct NodeInstance<I, O> {
    input_rx: Receiver<I>,
    output_txs: Vec<Sender<O>>,
    name: String,
}

impl<I: Send + 'static, O: Clone + Send + 'static> NodeInstance<I, O> {
    pub fn new(name: String, buffer_size: usize) -> (Self, Sender<I>) {
        let (tx, rx) = bounded(buffer_size);
        (
            NodeInstance {
                input_rx: rx,
                output_txs: Vec::new(),
                name,
            },
            tx,
        )
    }

    pub fn add_output(&mut self, buffer_size: usize) -> Receiver<O> {
        let (tx, rx) = bounded(buffer_size);
        self.output_txs.push(tx);
        rx
    }

    pub fn spawn<N>(self, mut node: N) -> NodeHandle
    where
        N: Node<Input = I, Output = O>,
    {
        let thread = thread::spawn(move || {
            node.on_start();

            // Err(_) == input channel is closed, shut down
            while let Ok(input) = self.input_rx.recv() {
                if let Some(output) = node.process(input) {
                    // Send to all connected outputs
                    for tx in &self.output_txs {
                        // If any output is disconnected, continue
                        if tx.send(output.clone()).is_err() {
                            break;
                        }
                    }
                }
            }

            node.on_stop();
        });

        NodeHandle {
            thread: Some(thread),
        }
    }
}
