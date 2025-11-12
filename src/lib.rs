pub mod node;

/// Connect nodes together using SPSC queue and a given bounded capacity.
/// Usage:
///  connect_nodes!(producer_node -> consumer_node, capacity);
#[macro_export]
macro_rules! connect_nodes {
    ( $tx:ident -> $rx:ident, $size:literal ) => {
        let (tx, rx) = crossbeam_channel::bounded($size);
        $tx.set_sender(tx);
        $rx.set_receiver(rx);
    };
}
