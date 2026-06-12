use std::env;
use std::thread;
use std::time::Duration;
use colored::*;
use sysinfo::{System, Networks};
use machine_info::Machine;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::cursor::{Hide, Show, MoveTo};
use crossterm::{ExecutableCommand, execute};
use std::io::{stdout, Write};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

fn main() {
    let args: Vec<String> = env::args().collect();

    enable_raw_mode().unwrap();
    stdout().execute(EnterAlternateScreen).unwrap();
    stdout().execute(Hide).unwrap();

    let mut targets = if args.len() < 2 {
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
    
    monitor_process(targets);
    stdout().execute(Show).unwrap();
    stdout().execute(LeaveAlternateScreen).unwrap();
    disable_raw_mode().unwrap();
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_000_000_000 {
        format!("{:.1}GB/s", bytes as f64 / 1_000_000_000.0)
    } else if bytes >= 1_000_000 {
        format!("{:.1}MB/s", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.1}KB/s", bytes as f64 / 1_000.0)
    } else {
        format!("{}B/s", bytes)
    }
}

fn get_bar(percentage: f32) -> String {
    let blocks = [" ", "\u{2581}", "\u{2582}", "\u{2583}", "\u{2584}", "\u{2585}", "\u{2586}", "\u{2587}", "\u{2588}"];
    let index = (((percentage.clamp(0.0, 100.0) / 100.0) * 8.0).ceil() as usize).clamp(1, 8);
    let bar = blocks[index];

    if percentage < 33.0 {
        bar.green().on_bright_black().to_string()
    } else if percentage < 66.0 {
        bar.yellow().on_bright_black().to_string()
    } else {
        bar.red().on_bright_black().to_string()
    }
}

fn monitor_process(targets: Vec<String>) {
    let mut sys = System::new_all();
    let machine = Machine::new();
    let mut networks = Networks::new_with_refreshed_list();

    let is_all = targets.iter().any(|t| t == "*");

    // Initial refresh and wait to ensure accurate first measurements
    sys.refresh_all();
    let num_cpus = sys.cpus().len().max(1) as f32;
    thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    
    let mut order = vec!["cpu", "mem"];

    'main_loop: loop {
        // Handle input - process all pending events
        while event::poll(Duration::from_millis(0)).unwrap() {
            if let Event::Key(key) = event::read().unwrap()
                && key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('c') => {
                            if order.contains(&"cpu") {
                                order.retain(|&x| x != "cpu");
                            } else {
                                order.push("cpu");
                            }
                        }
                        KeyCode::Char('m') => {
                            if order.contains(&"mem") {
                                order.retain(|&x| x != "mem");
                            } else {
                                order.push("mem");
                            }
                        }
                        KeyCode::Char('g') => {
                            if order.contains(&"gpu") {
                                order.retain(|&x| x != "gpu");
                            } else {
                                order.push("gpu");
                            }
                        }
                        KeyCode::Char('C') => {
                            order.retain(|&x| x != "cpu");
                            order.insert(0, "cpu");
                        }
                        KeyCode::Char('M') => {
                            order.retain(|&x| x != "mem");
                            order.insert(0, "mem");
                        }
                        KeyCode::Char('G') => {
                            order.retain(|&x| x != "gpu");
                            order.insert(0, "gpu");
                        }
                        KeyCode::Char('n') => {
                            if order.contains(&"net") {
                                order.retain(|&x| x != "net");
                            } else {
                                order.push("net");
                            }
                        }
                        KeyCode::Char('N') => {
                            order.retain(|&x| x != "net");
                            order.insert(0, "net");
                        }
                        KeyCode::Char('q') => break 'main_loop,
                        _ => {}
                    }
                }
        }

        // Refresh necessary components
        sys.refresh_cpu_usage();
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        sys.refresh_memory();
        networks.refresh(false);
        let gpu = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| machine.graphics_status())).unwrap_or_default();
        let (net_rx_ps, net_tx_ps): (u64, u64) = networks.list().values()
            .map(|n| (n.received(), n.transmitted()))
            .fold((0, 0), |acc, (r, t)| (acc.0 + r, acc.1 + t));
        // received()/transmitted() are bytes since last refresh (~500ms); multiply by 2 for bytes/sec
        let net_rx_ps = net_rx_ps * 2;
        let net_tx_ps = net_tx_ps * 2;

        let total_mem = sys.total_memory() as f32;

        let mut found = false;

        // Print header
        execute!(stdout(), MoveTo(0, 0), Clear(ClearType::FromCursorDown)).unwrap();

        let mut parts = Vec::new();
        for &metric in &order {
            match metric {
                "cpu" if is_all => {
                    let global_cpu = sys.global_cpu_usage();
                    parts.push(format!("CPU: {} {:.2}%", get_bar(global_cpu), global_cpu));
                }
                "mem" if is_all => {
                    let used_mem = sys.used_memory() as f32;
                    let mem_pct = if total_mem > 0.0 { (used_mem / total_mem) * 100.0 } else { 0.0 };
                    parts.push(format!("Mem: {} {:.2}% ({:.1}/{:.1} GB)",
                        get_bar(mem_pct), mem_pct,
                        used_mem / 1024.0 / 1024.0 / 1024.0,
                        total_mem / 1024.0 / 1024.0 / 1024.0
                    ));
                }
                "gpu" if is_all => {
                    if let Some(g) = gpu.first() {
                        parts.push(format!("GPU: {} {}% | Temp: {}°C | Mem: {}GB", get_bar(g.gpu as f32), g.gpu, g.temperature, g.memory_used / 1024 / 1024 / 1024));
                    } else {
                        parts.push("GPU: ?".to_string());
                    }
                }
                "net" if is_all => {
                    let total_bps = net_rx_ps + net_tx_ps;
                    let pct = (total_bps as f32 / 1_000_000.0).min(100.0);
                    parts.push(format!("Net: {} ↓{} ↑{}", get_bar(pct), format_bytes(net_rx_ps), format_bytes(net_tx_ps)));
                }
                _ => {}
            }
        }

        if !parts.is_empty() {
            let label = if is_all { "SYSTEM" } else { "GLOBAL" };
            print!("{} {}\r\n\r\n", label.bold().yellow(), parts.join(" | "));
        } else if is_all {
            print!("{} ?\r\n\r\n", "SYSTEM".bold().yellow());
        }

        let mut proc_list: Vec<_> = sys.processes().iter().collect();
        proc_list.sort_by(|a, b| b.1.cpu_usage().partial_cmp(&a.1.cpu_usage()).unwrap_or(std::cmp::Ordering::Equal));

        // Group matching processes by the target they matched
        let mut group_map: std::collections::HashMap<String, Vec<(String, &sysinfo::Process)>> = std::collections::HashMap::new();
        for (pid, proc) in &proc_list {
            let proc_name = proc.name().to_string_lossy();
            let pid_str = pid.to_string();
            let matched_target = targets.iter().filter(|t| *t != "*").find(|t| {
                proc_name.to_lowercase().contains(&t.to_lowercase()) || pid_str == **t
            }).map(|s| s.as_str());
            if let Some(t) = matched_target {
                group_map.entry(t.to_string()).or_default().push((pid_str, *proc));
            }
        }

        // Sort groups by total CPU descending
        let mut groups: Vec<_> = group_map.into_iter().collect();
        groups.sort_by(|a, b| {
            let cpu_a: f32 = a.1.iter().map(|(_, p)| p.cpu_usage()).sum();
            let cpu_b: f32 = b.1.iter().map(|(_, p)| p.cpu_usage()).sum();
            cpu_b.partial_cmp(&cpu_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        for (target, procs) in &groups {
            found = true;
            let count = procs.len();

            let mut output = if count == 1 {
                let (pid_str, proc) = &procs[0];
                let proc_name = proc.name().to_string_lossy();
                if pid_str == target {
                    format!("{} Name = {}", target.bold().green(), proc_name.green())
                } else {
                    format!("{} - {}: PID = {}, Name = {}", target.bold().green(), "Process".bold(), pid_str, proc_name.green())
                }
            } else {
                format!("{} {}", target.bold().green(), format!("({} PIDs)", count).dimmed())
            };

            let mut metric_added = false;
            for &metric in &order {
                match metric {
                    "cpu" => {
                        let total_cpu: f32 = procs.iter().map(|(_, p)| p.cpu_usage() / num_cpus).sum();
                        output.push_str(&format!(", CPU = {} {:.2}%", get_bar(total_cpu), total_cpu));
                        metric_added = true;
                    }
                    "mem" => {
                        let proc_mem_bytes: u64 = procs.iter().map(|(_, p)| p.memory()).sum();
                        let mem_pct = (proc_mem_bytes as f32 / total_mem) * 100.0;
                        output.push_str(&format!(", Mem = {} {:.2}%", get_bar(mem_pct), mem_pct));
                        metric_added = true;
                    }
                    "gpu" => {
                        let label = format!("GPU{}", ":G".truecolor(255, 165, 0));
                        if let Some(g) = gpu.first() {
                            output.push_str(&format!(", {} = {} {}%", label, get_bar(g.gpu as f32), g.gpu));
                        } else {
                            output.push_str(&format!(", {} = ?", label));
                        }
                        metric_added = true;
                    }
                    "net" => {
                        let label = format!("Net{}", ":N".truecolor(255, 165, 0));
                        let total_bps = net_rx_ps + net_tx_ps;
                        let pct = (total_bps as f32 / 1_000_000.0).min(100.0);
                        output.push_str(&format!(", {} = {} ↓{} ↑{}", label, get_bar(pct), format_bytes(net_rx_ps), format_bytes(net_tx_ps)));
                        metric_added = true;
                    }
                    _ => {}
                }
            }
            if !metric_added {
                output.push_str(" ?");
            }

            print!("{}\r\n", output);
        }

        if !found {
            let other_targets: Vec<_> = targets.iter().filter(|t| *t != "*").collect();
            if !other_targets.is_empty() {
                let targets_str = other_targets.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
                print!("{} - {}\r\n", targets_str.bold().green(), "Process not found or exited.".red());
            }
        }

        stdout().flush().unwrap();
        thread::sleep(Duration::from_millis(500));
    }
}

fn select_process(initial_query: Option<&str>) -> Vec<String> {
    let mut sys = System::new_all();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let mut query = initial_query.unwrap_or("").to_string();
    let matcher = SkimMatcherV2::default();
    let mut selected_index = 0;

    // Build name -> count map once (snapshot; no re-refresh during search)
    let mut name_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (_, proc) in sys.processes() {
        *name_counts.entry(proc.name().to_string_lossy().to_string()).or_insert(0) += 1;
    }

    loop {
        let mut matches: Vec<(i64, String, String)> = Vec::new(); // (score, target, display)

        // Add SYSTEM option
        if query.is_empty() {
             matches.push((0, "*".to_string(), "SYSTEM (All Processes)".to_string()));
        } else if let Some(score) = matcher.fuzzy_match("SYSTEM", &query) {
             matches.push((score, "*".to_string(), "SYSTEM (All Processes)".to_string()));
        }

        for (name, count) in &name_counts {
            let display = if *count > 1 {
                format!("{} ({} PIDs)", name, count)
            } else {
                name.clone()
            };
            if query.is_empty() {
                matches.push((0, name.clone(), display));
            } else if let Some(score) = matcher.fuzzy_match(name, &query) {
                matches.push((score, name.clone(), display));
            }
        }

        // Sort by score (desc), then display name
        matches.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.2.cmp(&b.2)));

        if !matches.is_empty() {
            selected_index = selected_index.min(matches.len() - 1);
        }

        execute!(stdout(), MoveTo(0, 0), Clear(ClearType::FromCursorDown)).unwrap();
        print!("{} {}\r\n", "Fuzzy Search:".bold().yellow(), query.cyan());
        print!("{}\r\n", "Use arrows to move, Enter to select, Esc to quit.".dimmed());
        print!("\r\n");

        if matches.is_empty() {
            print!("{}\r\n", "No matches found.".red());
        } else {
            let (_, term_height) = crossterm::terminal::size().unwrap_or((80, 24));
            let height = (term_height as usize).saturating_sub(4); // room for header
            let start = if selected_index >= height / 2 {
                (selected_index - height / 2).min(matches.len().saturating_sub(height))
            } else {
                0
            };
            let end = (start + height).min(matches.len());

            for (i, (_, _, display)) in matches.iter().enumerate().skip(start).take(end - start) {
                if i == selected_index {
                    print!("> {}\r\n", display.bold().green());
                } else {
                    print!("  {}\r\n", display);
                }
            }
        }
        
        stdout().flush().unwrap();

        if event::poll(Duration::from_millis(500)).unwrap()
            && let Event::Key(key) = event::read().unwrap()
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Esc => return Vec::new(),
                KeyCode::Enter if !matches.is_empty() => {
                    return vec![matches[selected_index].1.clone()];
                }
                KeyCode::Up => {
                    selected_index = selected_index.saturating_sub(1);
                }
                KeyCode::Down if selected_index + 1 < matches.len() => {
                    selected_index += 1;
                }
                KeyCode::Char(c) => {
                    query.push(c);
                    selected_index = 0;
                }
                KeyCode::Backspace => {
                    query.pop();
                    selected_index = 0;
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_bar_idle() {
        // 0% should show the smallest block to indicate idle state
        let bar = get_bar(0.0);
        assert!(bar.contains("\u{2581}"));
    }

    #[test]
    fn test_get_bar_full() {
        // 100% should show the full block
        let bar = get_bar(100.0);
        assert!(bar.contains("\u{2588}"));
    }

    #[test]
    fn test_get_bar_thresholds() {
        // Force colors on for testing so we can distinguish thresholds
        colored::control::set_override(true);
        
        // < 33.0 is Green
        let green = get_bar(32.0);
        // 33.0 to < 66.0 is Yellow
        let yellow = get_bar(34.0);
        // >= 66.0 is Red
        let red = get_bar(67.0);

        assert_ne!(green, yellow, "Green and Yellow should be different (color codes)");
        assert_ne!(yellow, red, "Yellow and Red should be different (color codes)");
        assert_ne!(green, red, "Green and Red should be different (color codes)");
        
        // Reset color override to avoid affecting other things (though it's a test process)
        colored::control::unset_override();
    }

    #[test]
    fn test_get_bar_clamping() {
        // Negative should be treated as 0% (idle block)
        let bar_neg = get_bar(-5.0);
        assert!(bar_neg.contains("\u{2581}"));

        // > 100% should be treated as 100% (full block)
        let bar_over = get_bar(105.0);
        assert!(bar_over.contains("\u{2588}"));
    }

    #[test]
    fn test_get_bar_steps() {
        // Verify character selection at different percentages
        // 0-12.5% should be index 1 (▂)
        assert!(get_bar(0.0).contains("\u{2581}"));
        assert!(get_bar(12.5).contains("\u{2581}"));
        
        // 12.6% should jump to index 2 (▃)
        assert!(get_bar(13.0).contains("\u{2582}"));
        
        // 50% should be index 4 (▅)
        assert!(get_bar(50.0).contains("\u{2584}"));
        
        // 100% should be index 8 (█)
        assert!(get_bar(100.0).contains("\u{2588}"));
    }
}
