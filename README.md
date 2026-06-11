# bittytop

A tiny, colorful process monitor for your terminal.

## Usage

### Monitor by Name or PID
Provide one or more names or PIDs as arguments:
```bash
# Monitor by name
bittytop firefox

# Monitor by PID
bittytop 1234

# Monitor multiple targets
bittytop pid1 process_name2 5678
```

### System Monitoring
Use `*` to monitor overall system CPU, Memory, and GPU usage:
```bash
bittytop "*"
```

### Interactive Selection
Run without arguments to enter a fuzzy search interface and pick a process:
```bash
bittytop
```
You can also find "SYSTEM" in the search list to monitor overall usage.

## Interactive Keys

While monitoring, you can use the following keys:

- `c`: Toggle CPU usage display
- `m`: Toggle Memory usage display
- `g`: Toggle GPU usage display (if available)
- `C`: Move CPU usage to the primary position
- `M`: Move Memory usage to the primary position
- `G`: Move GPU usage to the primary position
- `q`: Quit

> **Note**: When monitoring specific processes, GPU usage is marked with an orange `:G` tag (e.g., `GPU:G`). This indicates it is a **global** system-wide metric, as per-process GPU tracking is not currently supported.

## Installation

### From Source
```bash
cargo install --path .
```

### For Development
If you want to run it from your build directory:
```bash
# Symlink
ln -s "$(pwd)/target/release/bittytop" /usr/local/bin/bittytop

# Or add to PATH
export PATH="$PATH:$(pwd)/target/release"
```

