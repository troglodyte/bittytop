use crate::service::SystemData;
use crate::utils::{format_bytes, get_bar};
use colored::*;
use crossterm::cursor::MoveTo;
use crossterm::terminal::{Clear, ClearType};
use crossterm::queue;
use std::io::Write;
use std::collections::HashMap;

/// Prepares the terminal output buffer based on the collected system data and user settings.
/// It handles grouping, sorting, and formatting the process and system metrics into a terminal-ready buffer.
pub fn prepare_view(
    data: &SystemData,
    targets: &[String],
    order: &[&str],
    num_cpus: f32,
    sort_ascending: bool,
) -> Vec<u8> {
    let is_all = targets.iter().any(|t| t == "*");
    let total_mem = data.total_memory as f32;
    
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    buf.extend_from_slice(b"\x1b[?2026h");
    queue!(buf, MoveTo(0, 0)).unwrap();

    let mut parts = Vec::new();
    for &metric in order {
        match metric {
            "cpu" if is_all => {
                let global_cpu = data.global_cpu;
                parts.push(format!("CPU: {} {:.2}%", get_bar(global_cpu), global_cpu));
            }
            "mem" if is_all => {
                let used_mem = data.used_memory as f32;
                let mem_pct = if total_mem > 0.0 { (used_mem / total_mem) * 100.0 } else { 0.0 };
                parts.push(format!("Mem: {} {:.2}% ({:.1}/{:.1} GB)",
                    get_bar(mem_pct), mem_pct,
                    used_mem / 1024.0 / 1024.0 / 1024.0,
                    total_mem / 1024.0 / 1024.0 / 1024.0
                ));
            }
            "gpu" if is_all => {
                if let Some(g) = data.gpu_status.first() {
                    parts.push(format!("GPU: {} {}% | Temp: {}°C | Mem: {}GB", get_bar(g.gpu as f32), g.gpu, g.temperature, g.memory_used / 1024 / 1024 / 1024));
                } else {
                    parts.push("GPU: ?".to_string());
                }
            }
            "net" if is_all => {
                let total_bps = data.net_rx + data.net_tx;
                let pct = (total_bps as f32 / 1_000_000.0).min(100.0);
                parts.push(format!("Net: {} ↓{} ↑{}", get_bar(pct), format_bytes(data.net_rx), format_bytes(data.net_tx)));
            }
            _ => {}
        }
    }

    if !parts.is_empty() {
        let label = if is_all { "SYSTEM" } else { "GLOBAL" };
        write!(buf, "{} {}\x1b[K\r\n\x1b[K\r\n", label.bold().yellow(), parts.join(" | ")).unwrap();
    } else if is_all {
        write!(buf, "{} ?\x1b[K\r\n\x1b[K\r\n", "SYSTEM".bold().yellow()).unwrap();
    }

    // Group matching processes by the target they matched
    let mut group_map: HashMap<String, Vec<(&String, &crate::service::ProcessData)>> = HashMap::new();
    for proc in &data.processes {
        let matched_target = targets.iter().filter(|t| *t != "*").find(|t| {
            proc.name.to_lowercase().contains(&t.to_lowercase()) || proc.pid == **t
        });
        if let Some(t) = matched_target {
            group_map.entry(t.to_string()).or_default().push((&proc.pid, proc));
        }
    }

    // Sort groups by PID (or alphabetical) based on user preference
    let mut groups: Vec<_> = group_map.into_iter().collect();
    groups.sort_by(|a, b| {
        let a_val = a.0.parse::<u32>();
        let b_val = b.0.parse::<u32>();
        let cmp = match (a_val, b_val) {
            (Ok(ap), Ok(bp)) => ap.cmp(&bp),
            _ => a.0.cmp(&b.0),
        };
        if sort_ascending { cmp } else { cmp.reverse() }
    });

    let mut found = false;
    for (target, procs) in &groups {
        found = true;
        let count = procs.len();

        let mut bars = String::new();
        let mut stats = String::new();

        for &metric in order {
            match metric {
                "cpu" => {
                    let total_cpu: f32 = procs.iter().map(|(_, p)| p.cpu_usage / num_cpus).sum();
                    bars.push_str(&get_bar(total_cpu));
                    stats.push_str(&format!(" {:.1}%", total_cpu));
                }
                "mem" => {
                    let proc_mem_bytes: u64 = procs.iter().map(|(_, p)| p.memory).sum();
                    let mem_pct = (proc_mem_bytes as f32 / total_mem) * 100.0;
                    bars.push_str(&get_bar(mem_pct));
                    stats.push_str(&format!(" {:.1}%", mem_pct));
                }
                "gpu" => {
                    let label = ":G".truecolor(255, 165, 0);
                    if let Some(g) = data.gpu_status.first() {
                        bars.push_str(&get_bar(g.gpu as f32));
                        stats.push_str(&format!(" {}{}%", label, g.gpu));
                    } else {
                        bars.push_str(" ");
                        stats.push_str(&format!(" {}?", label));
                    }
                }
                "net" => {
                    let label = "->".truecolor(255, 165, 0);
                    // let total_bps = data.net_rx + data.net_tx;
                    // let pct = (total_bps as f32 / 1_000_000.0).min(100.0);
                    // bars.push_str(&get_bar(pct));
                    stats.push_str(&format!(" {} ↓{} ↑{}", label, format_bytes(data.net_rx), format_bytes(data.net_tx)));
                }
                _ => {}
            }
        }

        let name_part = if count == 1 {
            let (pid_str, proc) = &procs[0];
            if **pid_str == *target {
                format!("{} {}", target.bold().green(), proc.name.green())
            } else {
                format!("{} - {}: PID = {}, {}", target.bold().green(), "Process".bold(), pid_str, proc.name.green())
            }
        } else {
            format!("{} {}", target.bold().green(), format!("({} PIDs)", count).dimmed())
        };

        let prefix = if bars.is_empty() { "".to_string() } else { format!("{} ", bars) };
        let suffix = if stats.is_empty() { " ?".to_string() } else { stats };
        write!(buf, "{}{}{}\x1b[K\r\n", prefix, name_part, suffix).unwrap();
    }

    if !found {
        let other_targets: Vec<_> = targets.iter().filter(|t| *t != "*").collect();
        if !other_targets.is_empty() {
            let targets_str = other_targets.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
            write!(buf, "{} - {}\x1b[K\r\n", targets_str.bold().green(), "Process not found or exited.".red()).unwrap();
        }
    }

    queue!(buf, Clear(ClearType::FromCursorDown)).unwrap();
    buf.extend_from_slice(b"\x1b[?2026l");
    buf
}
