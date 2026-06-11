# bittytop
A tiny process monitor for your terminal.

## Usage
Monitor a process by name or PID:
```bash
bittytop my_process
```

# Installation 
### Add to your path
`cargo install --path .`

### For development
Symlink  
`ln -s "$(pwd)/target/release/bittytop" /usr/local/bin/bittytop`

Add to path  
`export PATH="$PATH:/path/to/your/bittytop-project/target/release"`

