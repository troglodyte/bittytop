# bittytop
A tiny process monitor for your terminal.

## Usage
Monitor a process by name or PID:
```bash
bittytop my_process
```

### Floating Mode
To keep bittytop visible at the bottom of your terminal while you continue working, use the `--float` (or `-f`) flag and run it in the background:
```bash
bittytop my_process --float &
```
This will reserve the last line of your terminal for bittytop. To stop it, use `fg` and Ctrl+C, or kill the process.

# Installation 
### Add to your path
`cargo install --path .`

### For development
Symlink  
`ln -s "$(pwd)/target/release/bittytop" /usr/local/bin/bittytop`

Add to path  
`export PATH="$PATH:/path/to/your/bittytop-project/target/release"`

