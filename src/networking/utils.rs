use colored::*;

/// Formats a byte count into a human-readable throughput string (B/s, KB/s, MB/s, GB/s).
pub fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_000_000_000 {
        format!("{:.1}GB/s", bytes as f64 / 1_000_000_000.0)
    } else if bytes >= 1_000_000 {
        format!("{:.1}MB/s", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.1}KB/s", bytes as f64 / 1_000.0)
    } else {
        format!("{}B/s", bytes)
    }
}

/// Generates a single-character colored bar graph representing a percentage value.
/// The bar changes color (green, yellow, red) based on the usage level.
pub fn get_bar(percentage: f32) -> String {
    let blocks = [" ", "\u{2581}", "\u{2582}", "\u{2583}", "\u{2584}", "\u{2585}", "\u{2586}", "\u{2587}", "\u{2588}"];
    let index = (((percentage.clamp(0.0, 100.0) / 100.0) * 8.0).ceil() as usize).clamp(1, 8);
    let bar = blocks[index];

    if percentage < 33.0 {
        bar.green().on_bright_black().to_string()
    } else if percentage < 66.0 {
        bar.yellow().on_bright_black().to_string()
    } else {
        bar.red().on_bright_black().to_string()
    }
}
