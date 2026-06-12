pub mod network;
pub mod service;
pub mod view;
pub mod monitor;
pub mod selector;
pub mod utils;
mod tests;

pub fn run(args: Vec<String>) -> bool {
    let mut show_net = false;
    let mut targets = if args.contains(&"--wtn".to_string()) {
        show_net = true;
        network::get_network_pids()
    } else {
        // This path might not be used if main.rs handles standard monitoring,
        // but it's good to have for completeness if we ever switch.
        if args.len() < 2 {
            selector::select_process(None)
        } else if args.len() == 2 && args[1].parse::<u32>().is_err() && args[1] != "*" {
            selector::select_process(Some(&args[1]))
        } else {
            args[1..].to_vec()
        }
    };

    if targets.is_empty() {
        return false;
    }

    // Heuristic: if multiple targets all exist as files and include common project items,
    // it's likely an unquoted shell expansion of '*'.
    if targets.len() > 1
        && targets.iter().all(|t| std::path::Path::new(t).exists())
        && targets.iter().any(|t| matches!(t.as_str(), "Cargo.toml" | "src" | "target" | "Cargo.lock"))
    {
        targets = vec!["*".to_string()];
    }
    
    monitor::monitor_process(targets, show_net);
    true
}
