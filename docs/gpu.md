# GPU Monitor Documentation

The formatting for GPU metrics is handled in **`src/view.rs`** within the `prepare_view` function. Note that GPU metrics are currently global (system-wide) as per-process GPU tracking is not yet implemented.

## 1. System-wide View
When monitoring the entire system (using `*`), the GPU load, temperature, and memory usage are displayed (Lines 42–48):

```rust
"gpu" if is_all => {
    if let Some(g) = data.gpu_status.first() {
        parts.push(format!("GPU: {} {}% | Temp: {}°C | Mem: {}GB", 
            get_bar(g.gpu as f32), g.gpu, g.temperature, g.memory_used / 1024 / 1024 / 1024));
    } else {
        parts.push("GPU: ?".to_string());
    }
}
```

## 2. Per-Process/Group View
When viewing specific processes, the GPU metric is marked with an orange `:G` tag to indicate it is a system-wide metric (Lines 109–118):

```rust
"gpu" => {
    let label = ":G".truecolor(255, 165, 0); // Orange label
    if let Some(g) = data.gpu_status.first() {
        // Bar prepended to the line
        bars.push_str(&get_bar(g.gpu as f32));
        
        // Status appended to the line
        stats.push_str(&format!(" {}{}%", label, g.gpu));
    } else {
        bars.push_str(" ");
        stats.push_str(&format!(" {}?", label));
    }
}
```
