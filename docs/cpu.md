# CPU Monitor Documentation

The formatting for CPU usage metrics is handled in **`src/view.rs`** within the `prepare_view` function.

## 1. System-wide View
When monitoring the entire system (using `*`), the global CPU usage is formatted in the header section (Lines 29–32):

```rust
"cpu" if is_all => {
    let global_cpu = data.global_cpu;
    parts.push(format!("CPU: {} {:.2}%", get_bar(global_cpu), global_cpu));
}
```

## 2. Per-Process/Group View
For specific processes or groups of processes, the CPU usage is calculated and normalized by the number of logical CPUs (Lines 98–102):

```rust
"cpu" => {
    let total_cpu: f32 = procs.iter().map(|(_, p)| p.cpu_usage / num_cpus).sum();
    
    // Bar prepended to the line
    bars.push_str(&get_bar(total_cpu));
    
    // Status appended to the line
    stats.push_str(&format!(" {:.1}%", total_cpu));
}
```

The usage is divided by `num_cpus` so that the percentage represents a value between 0% and 100% of the total system capacity, rather than per-core.
