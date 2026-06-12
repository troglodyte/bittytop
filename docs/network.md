# Network Monitor Documentation

The formatting for the `--wtn` command (and the network metric in general) is primarily handled in **`src/view.rs`** within the `prepare_view` function.

## Initial State for `--wtn`
In **`src/monitor.rs`**, when the `--wtn` flag is active, the metric display order is initialized to show only the network metric, and the default sort is descending (Lines 19–20):
```rust
let mut order = if show_net { vec!["net"] } else { vec!["cpu", "mem"] };
let mut sort_ascending = false;
```

## Interactive Sorting
The user can toggle sorting order using 'a' (ascending) and 'd' (descending) keys. This is handled in the main loop (Lines 28–33):
```rust
KeyCode::Char('a') => {
    sort_ascending = true;
}
KeyCode::Char('d') => {
    sort_ascending = false;
}
```

## Bar and Status Generation
In **`src/view.rs`**, the `prepare_view` function iterates through the `order` slice. For the `"net"` metric, it prepares the numeric status strings.

### Per-Process/Group Logic (Lines 119–125)
```rust
"net" => {
    let label = ":N".truecolor(255, 165, 0);
    let total_bps = data.net_rx + data.net_tx;
    let pct = (total_bps as f32 / 1_000_000.0).min(100.0);
    bars.push_str(&get_bar(pct));
    stats.push_str(&format!(" {} ↓{} ↑{}", label, format_bytes(data.net_rx), format_bytes(data.net_tx)));
}
```

### System-wide Logic (Lines 49–53)
```rust
"net" if is_all => {
    let total_bps = data.net_rx + data.net_tx;
    let pct = (total_bps as f32 / 1_000_000.0).min(100.0);
    parts.push(format!("Net: {} ↓{} ↑{}", get_bar(pct), format_bytes(data.net_rx), format_bytes(data.net_tx)));
}
```

## Sorting Logic
The sorting preference is applied in `src/view.rs` (Lines 76–86):
```rust
// Sort groups by PID (or alphabetical) based on user preference
let mut groups: Vec<_> = group_map.into_iter().collect();
groups.sort_by(|a, b| {
    let a_val = a.0.parse::<u32>();
    let b_val = b.0.parse::<u32>();
    let cmp = match (a_val, b_val) {
        (Ok(ap), Ok(bp)) => ap.cmp(&bp),
        _ => a.0.cmp(&b.0),
    };
    if sort_ascending { cmp } else { cmp.reverse() }
});
```

## Final Assembly
The final line is assembled at line 143 in `src/view.rs`:
```rust
write!(buf, "{}{}{}\x1b[K\r\n", prefix, name_part, suffix).unwrap();
```
