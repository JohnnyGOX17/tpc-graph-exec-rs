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
use crate::format_size;
use log::{debug, error, info, warn};
use rtrb::{Consumer, Producer};
use std::thread;
use std::time::{Duration, Instant};

/// Trait for types to report their memory footprint, used in throughput telemetry
pub trait DataSize {
    fn data_size(&self) -> usize;
}

impl<T: Sized> DataSize for Vec<T> {
    fn data_size(&self) -> usize {
        std::mem::size_of::<T>() * self.len()
    }
}

impl DataSize for () {
    fn data_size(&self) -> usize {
        0
    }
}

macro_rules! impl_data_size_primitive {
      ($($ty:ty),*) => {
          $(
              impl DataSize for $ty {
                  fn data_size(&self) -> usize {
                      std::mem::size_of_val(self)
                  }
              }
          )*
      };
  }

impl_data_size_primitive!(
    u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64, bool
);

/// Common trait of a Node
pub trait Node: Send + 'static {
    type Input: Send + 'static + DataSize;
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
    input_rx: Option<Consumer<I>>,
    /// Sender side of channel for Node's output
    output_tx: Option<Producer<O>>,
    /// Node name, used in naming OS thread
    name: String,
    /// Node to instantiate in spawned thread
    node: Box<dyn Node<Input = I, Output = O>>,
    /// Optional CPU core to pin spawned thread to
    cpu_core: Option<usize>,
    /// Bytes processed per telemetry period
    bytes_processed_cntr: u64,
}

impl<I: Send + 'static + DataSize, O: Clone + Send + 'static> NodeInstance<I, O> {
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
            bytes_processed_cntr: 0,
        }
    }

    /// Set input receiver channel
    pub fn set_receiver(&mut self, rx: Consumer<I>) {
        self.input_rx = Some(rx);
    }

    /// Set output transmitter channel
    pub fn set_sender(&mut self, tx: Producer<O>) {
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
                "Both input and output channels of node '{}' are missing! Thread will spawn a Node process with no data connections.",
                self.name
            );
        }

        thread::Builder::new().name(self.name.clone()).spawn(move || {
            if let Some(cpu_num) = self.cpu_core {
                let core_num = core_affinity::CoreId{id: cpu_num};
                if !core_affinity::set_for_current(core_num) {
                    warn!("Couldn't pin Node '{}' to CPU core {} (NOTE: this is expected on macOS)", self.name, cpu_num);
                }
            }

            if thread_priority::set_current_thread_priority(thread_priority::ThreadPriority::Max).is_err() {
                warn!("Couldn't set Node '{}' to maximum thread priority", self.name);

            };

            info!("Node '{}' starting", self.name);
            self.node.on_start();

            debug!("Node '{}' thread loop starting", self.name);
            // Telemetry vars
            let mut telem_time = Instant::now();
            let mut recv_time_acc = 0;
            let mut proc_time_acc = 0;
            let mut send_time_acc = 0;

            'main_loop: loop {
                let node_output = if let Some(input_ch) = self.input_rx.as_mut() {
                    // There exists some input channel for us to poll for new input data.
                    // rtrb's pop() is non-blocking, so we loop until data is available or the
                    // channel is closed (producer dropped). The OS scheduler puts the thread to
                    // sleep during yield when no data available and wakes it when scheduled.
                    let recv_time = Instant::now();
                    let rx_data = loop {
                        match input_ch.pop() {
                            Ok(data) => break data,
                            Err(_) => {
                                // Queue is empty - check if producer is still alive
                                if input_ch.is_abandoned() {
                                    // Producer dropped, channel is closed
                                    break 'main_loop;
                                }
                                // Queue just empty, yield to scheduler and try again
                                thread::yield_now();
                            }
                        }
                    };
                    recv_time_acc += recv_time.elapsed().as_nanos();
                    self.bytes_processed_cntr += rx_data.data_size() as u64;
                    let proc_time = Instant::now();
                    let retval = self.node.process(Some(rx_data));
                    proc_time_acc += proc_time.elapsed().as_nanos();
                    retval
                } else {
                    // No input channel given, assumed intentional and data produced internal to
                    // Node's process() logic
                    self.node.process(None)
                };

                // If there is no Node process() output, assume this node is intentionally sinking
                // data, or thread wait/sleep/yield has already occurred in above RX loops
                if let Some(tx_data) = node_output {
                    let send_time = Instant::now();
                    let output_ch = self.output_tx
                        .as_mut()
                        .expect("node produced data, so output channel should be connected, to not black-hole data");

                    // rtrb's push() is non-blocking, so we loop until space is available or the
                    // channel is closed (consumer dropped). On failure, push() returns the value.
                    let mut data = tx_data;
                    loop {
                        match output_ch.push(data) {
                            Ok(_) => break,
                            Err(rtrb::PushError::Full(returned_data)) => {
                                data = returned_data;
                                // Queue is full - check if consumer is still alive
                                if output_ch.is_abandoned() {
                                    // Consumer dropped, channel is closed
                                    break 'main_loop;
                                }
                                // Queue just full, yield to scheduler and try again
                                thread::yield_now();
                            }
                        }
                    }
                    send_time_acc += send_time.elapsed().as_nanos();
                }

                if telem_time.elapsed() >= Duration::from_secs(1) {
                    let total_time_ns = (recv_time_acc + proc_time_acc + send_time_acc) as f32;
                    let percent_recv = 100.0 * (recv_time_acc as f32) / total_time_ns;
                    let percent_proc = 100.0 * (proc_time_acc as f32) / total_time_ns;
                    let percent_send = 100.0 * (send_time_acc as f32) / total_time_ns;

                    info!("{} recv() throughput: {}/sec | RX wait: {:.2}%, Process wait: {:.2}%, TX wait: {:.2}%",
                        self.name,
                        format_size(self.bytes_processed_cntr as f32),
                        percent_recv,
                        percent_proc,
                        percent_send
                    );

                    self.bytes_processed_cntr = 0;
                    telem_time = Instant::now();
                    recv_time_acc = 0;
                    proc_time_acc = 0;
                    send_time_acc = 0;
                }
            }

            info!("Node '{}' stopping", self.name);
            self.node.on_stop();
        })
    }
}
