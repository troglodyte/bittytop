use std::env;
use std::thread;
use std::time::Duration;
use colored::*;
use sysinfo::System;
use machine_info::Machine;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::cursor::{Hide, Show};
use crossterm::ExecutableCommand;
use std::io::stdout;

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut targets = if args.len() < 2 {
        vec!["*".to_string()]
    } else {
        args[1..].to_vec()
    };

    // Heuristic: if multiple targets all exist as files and include common project items,
    // it's likely an unquoted shell expansion of '*'.
    if targets.len() > 1
        && targets.iter().all(|t| std::path::Path::new(t).exists())
        && targets.iter().any(|t| matches!(t.as_str(), "Cargo.toml" | "src" | "target" | "Cargo.lock"))
    {
        targets = vec!["*".to_string()];
    }
    
    enable_raw_mode().unwrap();
    stdout().execute(Hide).unwrap();
    monitor_process(targets);
    stdout().execute(Show).unwrap();
    disable_raw_mode().unwrap();
}

fn get_bar(percentage: f32) -> String {
    let blocks = [" ", "\u{2581}", "\u{2582}", "\u{2583}", "\u{2584}", "\u{2585}", "\u{2586}", "\u{2587}", "\u{2588}"];
    let index = (((percentage.clamp(0.0, 100.0) / 100.0) * 8.0).ceil() as usize).max(1).min(8);
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
    
    let is_all = targets.iter().any(|t| t == "*");
    
    // Initial refresh and wait to ensure accurate first measurements
    sys.refresh_all();
    let num_cpus = sys.cpus().len().max(1) as f32;
    thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    
    let mut show_cpu = true;
    let mut show_mem = true;
    let mut show_gpu = false;

    loop {
        // Handle input
        if event::poll(Duration::from_millis(100)).unwrap() {
            if let Event::Key(key) = event::read().unwrap() {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('c') => show_cpu = !show_cpu,
                        KeyCode::Char('m') => show_mem = !show_mem,
                        KeyCode::Char('g') => show_gpu = !show_gpu,
                        KeyCode::Char('q') => break,
                        _ => {}
                    }
                }
            }
        }

        // Refresh necessary components
        sys.refresh_cpu_usage();
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        sys.refresh_memory();
        let gpu = machine.graphics_status(); 

        let mut found = false;
        
        // Print header
        print!("{}[2J{}[1;1H", 27 as char, 27 as char); // Clear screen

        let mut header = String::new();
        if is_all {
            let mut system_parts = Vec::new();
            if show_cpu {
                let global_cpu = sys.global_cpu_usage();
                system_parts.push(format!("CPU: {} {:.2}%", get_bar(global_cpu), global_cpu));
            }
            if show_mem {
                let total_mem = sys.total_memory() as f32;
                let used_mem = sys.used_memory() as f32;
                let mem_pct = if total_mem > 0.0 { (used_mem / total_mem) * 100.0 } else { 0.0 };
                system_parts.push(format!("Mem: {} {:.2}% ({:.1}/{:.1} GB)",
                    get_bar(mem_pct), mem_pct,
                    used_mem / 1024.0 / 1024.0 / 1024.0,
                    total_mem / 1024.0 / 1024.0 / 1024.0
                ));
            }

            if !system_parts.is_empty() {
                header.push_str(&format!("{} ", "SYSTEM".bold().yellow()));
                header.push_str(&system_parts.join(" | "));
            } else if show_gpu {
                header.push_str(&format!("{}", "SYSTEM".bold().yellow()));
            }
        }

        if show_gpu {
            if !header.is_empty() {
                header.push_str(" | ");
            }
            if let Some(g) = gpu.first() {
                header.push_str(&format!("GPU: {} {}% | Temp: {}°C | Mem: {}GB", get_bar(g.gpu as f32), g.gpu, g.temperature, g.memory_used / 1024 / 1024 / 1024));
            } else {
                 header.push_str("GPU: ?");
            }
        }

        if !header.is_empty() {
            println!("{}", header);
            println!();
        }

        let total_mem = sys.total_memory() as f32;
        let mut proc_list: Vec<_> = sys.processes().iter().collect();
        proc_list.sort_by(|a, b| b.1.cpu_usage().partial_cmp(&a.1.cpu_usage()).unwrap_or(std::cmp::Ordering::Equal));

        for (pid, proc) in proc_list {
            let proc_name = proc.name().to_string_lossy();
            let pid_str = pid.to_string();
            
            let matched_target = targets.iter().filter(|t| *t != "*").find(|t| {
                proc_name.to_lowercase().contains(&t.to_lowercase()) || pid_str == **t
            }).map(|s| s.as_str());

            if let Some(t) = matched_target {
                let mut output = format!("{} - {}: PID = {}, Name = {}", t.bold().green(), "Process".bold(), pid, proc_name.green());
                
                if show_cpu {
                    let cpu_usage = proc.cpu_usage() / num_cpus;
                    output.push_str(&format!(", CPU = {} {:.2}%", get_bar(cpu_usage), cpu_usage));
                }
                if show_mem {
                    let mem_pct = (proc.memory() as f32 / total_mem) * 100.0;
                    output.push_str(&format!(", Mem = {} {:.2}%", get_bar(mem_pct), mem_pct));
                }
                
                println!("{}", output);
                found = true;
            }
        }

        if !found {
            let other_targets: Vec<_> = targets.iter().filter(|t| *t != "*").collect();
            if !other_targets.is_empty() {
                let targets_str = other_targets.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
                println!("{} - {}", targets_str.bold().green(), "Process not found or exited.".red());
            }
        }

        thread::sleep(Duration::from_millis(500));
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
