use std::process::Command;
use std::collections::HashSet;

/// Discovers and returns a list of PIDs that currently have active network connections.
/// It uses the `lsof -i -n -P` command and returns the PIDs sorted in descending order.
pub fn get_network_pids() -> Vec<String> {
    let output = Command::new("lsof")
        .args(&["-i", "-n", "-P"])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut pids = HashSet::new();
            for line in stdout.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() > 1 {
                    pids.insert(parts[1].to_string());
                }
            }
            let mut result: Vec<String> = pids.into_iter().collect();
            result.sort_by(|a, b| {
                let a_num = a.parse::<u32>().unwrap_or(0);
                let b_num = b.parse::<u32>().unwrap_or(0);
                b_num.cmp(&a_num)
            });
            result
        }
        _ => Vec::new(),
    }
}
