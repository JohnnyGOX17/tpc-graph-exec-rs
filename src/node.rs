//! # Node
//!
//! A node consists of a main `process` (executed continuously within a dedicated
//! thread), and `on_start()` and `on_stop()` methods called at the creation and destruction of the
//! Node, respectively. For simplicity, and targeted dataplane/compute-intensive applications, only
//! Single-Producer Single-Consumer (SPSC) queue constructs are used. This also simplifies each
//! node to only have a single input and output type specification with no added synchronization
//! logic. Bounded queues are used for lock-free, cache-friendly, performant operation.
//!
//! If you need parallelism for compute-heavy stages (like a big FFT), you're often better off with
//! vectorization or splitting the work within a stage rather than adding queue complexity.
use crossbeam_channel::{Receiver, Sender};
use log::{debug, error, info, warn};
use std::thread;

/// Common trait of a Node
pub trait Node: Send + 'static {
    type Input: Send + 'static;
    type Output: Send + 'static;

    /// Process a single input and produce an output
    fn process(&mut self, input: Option<Self::Input>) -> Option<Self::Output>;

    /// Called once when the node starts
    fn on_start(&mut self) {}

    /// Called once when the node stops
    fn on_stop(&mut self) {}
}

/// A node instance in the graph with its channels
pub struct NodeInstance<I, O> {
    /// Receiver side of channel for Node's input
    input_rx: Option<Receiver<I>>,
    /// Sender side of channel for Node's output
    output_tx: Option<Sender<O>>,
    /// Node name, used in naming OS thread
    name: String,
    /// Node to instantiate in spawned thread
    node: Box<dyn Node<Input = I, Output = O>>,
    /// Optional CPU core to pin spawned thread to
    cpu_core: Option<usize>,
}

impl<I: Send + 'static, O: Clone + Send + 'static> NodeInstance<I, O> {
    /// Create a new NodeInstance with a given name and Node
    pub fn new<N>(name: String, node: N, cpu_core: Option<usize>) -> Self
    where
        N: Node<Input = I, Output = O>,
    {
        NodeInstance {
            input_rx: None,
            output_tx: None,
            name,
            node: Box::new(node),
            cpu_core,
        }
    }

    /// Set input receiver channel
    pub fn set_receiver(&mut self, rx: Receiver<I>) {
        self.input_rx = Some(rx);
    }

    /// Set output transmitter channel
    pub fn set_sender(&mut self, tx: Sender<O>) {
        self.output_tx = Some(tx);
    }

    /// Spawn and start new OS thread with Node logic, returning handle to thread
    pub fn spawn(mut self) -> Result<thread::JoinHandle<()>, std::io::Error> {
        if self.input_rx.is_none() {
            warn!(
                "No input (RX) channel connected to node '{}' (this may be intentional)",
                self.name
            );
        }
        if self.output_tx.is_none() {
            warn!(
                "No output (TX) channel connected to node '{}' (this may be intentional)",
                self.name
            );
        }

        if self.output_tx.is_none() && self.input_rx.is_none() {
            error!(
                "Both input and output channels of node '{}' are missing! Thread will spawn a Node thread with no data connections.",
                self.name
            );
        }

        thread::Builder::new().name(self.name.clone()).spawn(move || {
            if let Some(cpu_num) = self.cpu_core {
                let core_num = core_affinity::CoreId{id: cpu_num};
                if !core_affinity::set_for_current(core_num) {
                    warn!("Couldn't pin Node '{}' to CPU core {}", self.name, cpu_num);
                }
            }

            info!("Node '{}' starting", self.name);
            self.node.on_start();

            debug!("Node '{}' thread loop starting", self.name);
            loop {
                let node_output = if let Some(input_ch) = self.input_rx.as_ref() {
                    // There exists some input channel for us to poll for new input data. Use
                    // blocking receive method (waits till data is available) instead of
                    // `try_recv()`- the OS scheduler puts the thread to sleep when no data
                    // available and wakes it when data arrives. No CPU usage while waiting, near
                    // instant wakeup.
                    match input_ch.recv() {
                        Ok(rx_data) => self.node.process(Some(rx_data)),
                        Err(_) => break, // channel closed
                    }
                } else {
                    // No input channel given, assumed intentional and data produced internal to
                    // Node's process() logic
                    self.node.process(None)
                };

                // If there is no Node process() output, assume this node is intentionally sinking
                // data, or thread wait/sleep/yield has already occurred in above RX loops
                if let Some(tx_data) = node_output {
                    // Err(_) == channel is closed, shut down
                    if self.output_tx
                        .as_ref()
                        .expect("node produced data, so output channel should be connected, to not black-hole data")
                        .send(tx_data)
                        .is_err() {
                        break;
                    }
                }
            }

            info!("Node '{}' stopping", self.name);
            self.node.on_stop();
        })
    }
}
