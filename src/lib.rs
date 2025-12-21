pub mod node;

/// Connect nodes together using SPSC queue and a given bounded capacity.
/// Usage:
///  connect_nodes!(producer_node -> consumer_node, capacity);
#[macro_export]
macro_rules! connect_nodes {
    ( $tx:ident -> $rx:ident, $size:literal ) => {
        let (tx, rx) = rtrb::RingBuffer::new($size);
        $tx.set_sender(tx);
        $rx.set_receiver(rx);
    };
}

/// Convert number of bytes to formatted string
pub fn format_size(bytes: f32) -> String {
    const GB: f32 = 1024.0 * 1024.0 * 1024.0;
    const MB: f32 = 1024.0 * 1024.0;
    const KB: f32 = 1024.0;

    if bytes >= GB {
        format!("{:.2} GiB", bytes / GB)
    } else if bytes >= MB {
        format!("{:.2} MiB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.2} KiB", bytes / KB)
    } else {
        format!("{:.2} B", bytes)
    }
}

/// Convert number of bytes to formatted string for fixed-width integer string
pub fn format_size_fixed_int(bytes: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;
    const KB: u64 = 1024;

    if bytes >= GB {
        format!("{:>4} GiB", bytes / GB)
    } else if bytes >= MB {
        format!("{:>4} MiB", bytes / MB)
    } else if bytes >= KB {
        format!("{:>4} KiB", bytes / KB)
    } else {
        format!("{:>4} B", bytes)
    }
}
