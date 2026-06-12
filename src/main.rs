mod utils;
mod network;
mod selector;
mod service;
mod view;
mod monitor;
mod tests;

use std::env;
use std::io::stdout;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::cursor::{Hide, Show};
use crossterm::ExecutableCommand;

use network::get_network_pids;
use selector::select_process;
use monitor::monitor_process;

/// The entry point of the application. It handles CLI arguments, sets up the terminal
/// into raw mode and alternate screen, and orchestrates the process selection or monitoring.
fn main() {
    let args: Vec<String> = env::args().collect();

    enable_raw_mode().unwrap();
    stdout().execute(EnterAlternateScreen).unwrap();
    stdout().execute(Hide).unwrap();

    let mut show_net = false;
    let mut targets = if args.contains(&"--wtn".to_string()) {
        show_net = true;
        get_network_pids()
    } else if args.len() < 2 {
        select_process(None)
    } else if args.len() == 2 && args[1].parse::<u32>().is_err() && args[1] != "*" {
        select_process(Some(&args[1]))
    } else {
        args[1..].to_vec()
    };

    if targets.is_empty() {
        stdout().execute(Show).unwrap();
        stdout().execute(LeaveAlternateScreen).unwrap();
        disable_raw_mode().unwrap();
        if show_net {
            println!("No processes found with active network connections.");
        }
        return;
    }

    // Heuristic: if multiple targets all exist as files and include common project items,
    // it's likely an unquoted shell expansion of '*'.
    if targets.len() > 1
        && targets.iter().all(|t| std::path::Path::new(t).exists())
        && targets.iter().any(|t| matches!(t.as_str(), "Cargo.toml" | "src" | "target" | "Cargo.lock"))
    {
        targets = vec!["*".to_string()];
    }
    
    monitor_process(targets, show_net);
    stdout().execute(Show).unwrap();
    stdout().execute(LeaveAlternateScreen).unwrap();
    disable_raw_mode().unwrap();
}
