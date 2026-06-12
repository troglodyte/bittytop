use crate::networking::service::SystemData;
use crate::networking::utils::{format_bytes, get_bar};
use colored::*;
use crossterm::cursor::MoveTo;
use crossterm::terminal::{Clear, ClearType};
use crossterm::queue;
use std::io::Write;
use std::collections::HashMap;

/// Prepares the terminal output buffer based on the collected system data and user settings for the standard monitor view.
pub fn prepare_monitor_view(
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
            "cpu" => {
                let global_cpu = data.global_cpu;
                parts.push(format!("CPU: {} {:.2}%", get_bar(global_cpu), global_cpu));
            }
            "mem" => {
                let used_mem = data.used_memory as f32;
                let mem_pct = if total_mem > 0.0 { (used_mem / total_mem) * 100.0 } else { 0.0 };
                parts.push(format!("Mem: {} {:.2}% ({:.1}/{:.1} GB)",
                    get_bar(mem_pct), mem_pct,
                    used_mem / 1024.0 / 1024.0 / 1024.0,
                    total_mem / 1024.0 / 1024.0 / 1024.0
                ));
            }
            "gpu" => {
                if let Some(g) = data.gpu_status.first() {
                    parts.push(format!("GPU: {} {}% | Temp: {}°C | Mem: {}GB", get_bar(g.gpu as f32), g.gpu, g.temperature, g.memory_used / 1024 / 1024 / 1024));
                }
            }
            "net" => {
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
    }

    // Group matching processes
    let mut group_map: HashMap<String, Vec<(&String, &crate::networking::service::ProcessData)>> = HashMap::new();
    if is_all {
        // For SYSTEM view, we group by name to keep it readable
        for proc in &data.processes {
            group_map.entry(proc.name.clone()).or_default().push((&proc.pid, proc));
        }
    } else {
        for proc in &data.processes {
            let matched_target = targets.iter().filter(|t| *t != "*").find(|t| {
                proc.name.to_lowercase().contains(&t.to_lowercase()) || proc.pid == **t
            });
            if let Some(t) = matched_target {
                group_map.entry(t.to_string()).or_default().push((&proc.pid, proc));
            }
        }
    }

    let mut groups: Vec<_> = group_map.into_iter().collect();
    
    // For monitor view, sort groups by the first metric's total usage if in SYSTEM view, 
    // otherwise stick to PID/Name sorting.
    if is_all && !order.is_empty() {
        groups.sort_by(|(_, a_procs), (_, b_procs)| {
            let cmp = match order[0] {
                "cpu" => {
                    let a_cpu: f32 = a_procs.iter().map(|(_, p)| p.cpu_usage).sum();
                    let b_cpu: f32 = b_procs.iter().map(|(_, p)| p.cpu_usage).sum();
                    a_cpu.partial_cmp(&b_cpu).unwrap_or(std::cmp::Ordering::Equal)
                }
                "mem" => {
                    let a_mem: u64 = a_procs.iter().map(|(_, p)| p.memory).sum();
                    let b_mem: u64 = b_procs.iter().map(|(_, p)| p.memory).sum();
                    a_mem.cmp(&b_mem)
                }
                _ => std::cmp::Ordering::Equal,
            };
            if sort_ascending { cmp } else { cmp.reverse() }
        });
        // Limit to top 20 for readability in SYSTEM view
        groups.truncate(20);
    } else {
        groups.sort_by(|a, b| {
            let a_val = a.0.parse::<u32>();
            let b_val = b.0.parse::<u32>();
            let cmp = match (a_val, b_val) {
                (Ok(ap), Ok(bp)) => ap.cmp(&bp),
                _ => a.0.cmp(&b.0),
            };
            if sort_ascending { cmp } else { cmp.reverse() }
        });
    }

    for (target, procs) in &groups {
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
                    if let Some(g) = data.gpu_status.first() {
                        bars.push_str(&get_bar(g.gpu as f32));
                        stats.push_str(&format!(" :G{}%", g.gpu));
                    }
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
        write!(buf, "{}{}{}\x1b[K\r\n", prefix, name_part, stats).unwrap();
    }

    if groups.is_empty() && !is_all {
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

pub fn prepare_wtn_view(
    data: &SystemData,
    targets: &[String],
    sort_ascending: bool,
) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    buf.extend_from_slice(b"\x1b[?2026h");
    queue!(buf, MoveTo(0, 0)).unwrap();

    let total_bps = data.net_rx + data.net_tx;
    let pct = (total_bps as f32 / 1_000_000.0).min(100.0);
    write!(buf, "{} Net: {} ↓{} ↑{}\x1b[K\r\n\x1b[K\r\n", 
        "NETWORK".bold().yellow(),
        get_bar(pct), 
        format_bytes(data.net_rx), 
        format_bytes(data.net_tx)
    ).unwrap();

    let mut procs_to_show = Vec::new();
    for target_pid in targets {
        if let Some(proc) = data.processes.iter().find(|p| p.pid == *target_pid) {
            procs_to_show.push(proc);
        }
    }

    procs_to_show.sort_by(|a, b| {
        let a_val = a.pid.parse::<u32>().unwrap_or(0);
        let b_val = b.pid.parse::<u32>().unwrap_or(0);
        if sort_ascending { a_val.cmp(&b_val) } else { b_val.cmp(&a_val) }
    });

    for proc in procs_to_show {
        let label = "->".truecolor(255, 165, 0);
        write!(buf, "{} {} {}: PID = {}, {}\x1b[K\r\n",
            get_bar(0.0), // Placeholder bar
            label,
            "Active".bold().green(),
            proc.pid.bold().green(),
            proc.name.green()
        ).unwrap();
    }

    if targets.is_empty() {
        write!(buf, "{}\x1b[K\r\n", "No processes with active network connections.".red()).unwrap();
    }

    queue!(buf, Clear(ClearType::FromCursorDown)).unwrap();
    buf.extend_from_slice(b"\x1b[?2026l");
    buf
}
