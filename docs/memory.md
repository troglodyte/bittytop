# Memory Monitor Documentation

The formatting for Memory usage metrics is handled in **`src/view.rs`** within the `prepare_view` function.

## 1. System-wide View
When monitoring the entire system (using `*`), memory usage is shown with both absolute values (GB) and percentage (Lines 33–41):

```rust
"mem" if is_all => {
    let used_mem = data.used_memory as f32;
    let mem_pct = if total_mem > 0.0 { (used_mem / total_mem) * 100.0 } else { 0.0 };
    parts.push(format!("Mem: {} {:.2}% ({:.1}/{:.1} GB)",
        get_bar(mem_pct), mem_pct,
        used_mem / 1024.0 / 1024.0 / 1024.0,
        total_mem / 1024.0 / 1024.0 / 1024.0
    ));
}
```

## 2. Per-Process/Group View
For specific processes or groups, the aggregate memory usage is shown as a percentage of the total system memory (Lines 103–108):

```rust
"mem" => {
    let proc_mem_bytes: u64 = procs.iter().map(|(_, p)| p.memory).sum();
    let mem_pct = (proc_mem_bytes as f32 / total_mem) * 100.0;
    
    // Bar prepended to the line
    bars.push_str(&get_bar(mem_pct));
    
    // Status appended to the line
    stats.push_str(&format!(" {:.1}%", mem_pct));
}
```
