use std::env;
use std::thread;
use std::time::Duration;
use colored::*;
use sysinfo::System;
use machine_info::Machine;
use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, size};
use std::io::{stdout, Write};
use crossterm::cursor::{MoveTo, SavePosition, RestorePosition};
use crossterm::queue;
use crossterm::style::Print;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <pid_or_name> [--float]", args[0].bold());
        return;
    }

    let target = &args[1];
    let is_float = args.contains(&"--float".to_string()) || args.contains(&"-f".to_string());

    if is_float {
        monitor_process_float(target);
    } else {
        let _ = enable_raw_mode();
        monitor_process(target);
        let _ = disable_raw_mode();
    }
}

fn truncate_ansi(s: &str, max_width: usize) -> String {
    let mut result = String::new();
    let mut width = 0;
    let mut in_ansi = false;
    let mut ansi_buf = String::new();

    if max_width < 4 { return s.to_string(); }

    for c in s.chars() {
        if c == '\x1b' {
            in_ansi = true;
            ansi_buf.push(c);
        } else if in_ansi {
            ansi_buf.push(c);
            if c >= '@' && c <= '~' {
                in_ansi = false;
                result.push_str(&ansi_buf);
                ansi_buf.clear();
            }
        } else {
            if width < max_width - 3 {
                result.push(c);
                width += 1;
            } else {
                result.push_str("\x1b[0m...");
                return result;
            }
        }
    }
    result
}

fn visible_len(s: &str) -> usize {
    let mut len = 0;
    let mut in_ansi = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_ansi = true;
        } else if in_ansi {
            if c >= '@' && c <= '~' {
                in_ansi = false;
            }
        } else {
            len += 1;
        }
    }
    len
}

fn get_bar(percentage: f32, width: usize) -> String {
    if width < 3 { return "".to_string(); }
    let bar_width = width - 2; // account for []
    let filled = (percentage.clamp(0.0, 100.0) / 100.0 * bar_width as f32).round() as usize;
    let empty = bar_width - filled;
    format!("[{}{}]", "█".repeat(filled).cyan(), "░".repeat(empty))
}

fn monitor_process_float(target: &str) {
    let mut sys = System::new_all();
    let machine = Machine::new();
    let mut stdout = stdout();

    // Set up Ctrl+C handler to reset scroll region and restore cursor
    let _ = ctrlc::set_handler(move || {
        let mut out = std::io::stdout();
        let _ = queue!(out, Print("\x1b[r"), RestorePosition);
        let _ = out.flush();
        std::process::exit(0);
    });

    // Initial refresh
    sys.refresh_all();

    let mut last_rows = 0;
    let mut last_cols = 0;

    loop {
        // Re-check size in case of resize
        let (cols, rows) = size().unwrap_or((80, 24));
        
        if rows != last_rows || cols != last_cols {
            // Set scroll region to exclude the last line
            // Save cursor position because DECSTBM moves it to (1,1)
            let _ = queue!(stdout, 
                SavePosition,
                Print(format!("\x1b[1;{}r", rows - 1)),
                RestorePosition
            );
            let _ = stdout.flush();
            last_rows = rows;
            last_cols = cols;
        }

        sys.refresh_all();
        let gpu = machine.graphics_status(); 

        let mut found = false;
        let mut status_line = String::new();

        for (pid, proc) in sys.processes() {
            let proc_name = proc.name().to_string_lossy();
            let pid_str = pid.to_string();
            
            if proc_name.to_lowercase().contains(&target.to_lowercase()) || pid_str == target {
                let cpu_usage = proc.cpu_usage();
                let mem_pct = (proc.memory() as f32 / sys.total_memory() as f32) * 100.0;
                
                status_line = format!("{} {} | PID: {} | CPU: {:.1}% | Mem: {:.1}%", 
                    "bittytop:".bold().blue(), target.bold().green(), pid, cpu_usage, mem_pct);
                
                if let Some(g) = gpu.first() {
                    status_line.push_str(&format!(" | GPU: {}% {}°C", g.gpu, g.temperature));
                }

                found = true;
                break; // Just show the first match for float mode
            }
        }

        if !found {
            status_line = format!("{} {} - {}", "bittytop:".bold().blue(), target.bold().green(), "Process not found or exited.".red());
        }

        // Robust truncation and padding
        let truncated = truncate_ansi(&status_line, cols as usize);
        let vlen = visible_len(&truncated);
        let padding = (cols as usize).saturating_sub(vlen);
        let final_line = format!("{}{}", truncated, " ".repeat(padding));

        // Move to last line, print status, restore cursor
        let _ = queue!(stdout, 
            SavePosition,
            MoveTo(0, rows - 1),
            Print(final_line),
            RestorePosition
        );
        let _ = stdout.flush();

        thread::sleep(Duration::from_millis(1000));
    }
}

fn monitor_process(target: &str) {
    let mut sys = System::new_all();
    let machine = Machine::new();
    
    // Initial refresh
    sys.refresh_all();
    
    let mut show_cpu = true;
    let mut show_mem = true;
    let mut show_gpu = false;

    loop {
        let (cols, rows) = size().unwrap_or((80, 24));

        // Handle input
        if event::poll(Duration::from_millis(100)).unwrap() {
            if let Event::Key(key) = event::read().unwrap() {
                match key.code {
                    KeyCode::Char('c') => show_cpu = !show_cpu,
                    KeyCode::Char('m') => show_mem = !show_mem,
                    KeyCode::Char('g') => show_gpu = !show_gpu,
                    KeyCode::Char('q') => break,
                    _ => {}
                }
            }
        }

        // Refresh everything
        sys.refresh_all();
        let gpu = machine.graphics_status(); 

        let mut matches = Vec::new();
        for (pid, proc) in sys.processes() {
            let proc_name = proc.name().to_string_lossy();
            let pid_str = pid.to_string();
            if proc_name.to_lowercase().contains(&target.to_lowercase()) || pid_str == target {
                matches.push((pid, proc));
            }
        }
        
        // Print header
        print!("{}[2J{}[1;1H", 27 as char, 27 as char); // Clear screen
        
        let mut lines_available = rows as usize;
        
        if show_gpu {
            if let Some(g) = gpu.first() {
                println!("{} GPU: {} {}% | Temp: {}°C | Mem: {}GB", "bittytop:".bold().blue(), get_bar(g.gpu as f32, 10), g.gpu, g.temperature, g.memory_used / 1024 / 1024 / 1024);
                lines_available = lines_available.saturating_sub(1);
            } else {
                 println!("{} GPU: N/A", "bittytop:".bold().blue());
                 lines_available = lines_available.saturating_sub(1);
            }
            println!();
            lines_available = lines_available.saturating_sub(1);
        }

        if matches.is_empty() {
            println!("{} {} - {}", "bittytop:".bold().blue(), target.bold().green(), "Process not found or exited.".red());
        } else {
            // Adjust bar width based on terminal width
            let bar_width = if cols > 80 { 15 } else if cols > 40 { 10 } else { 5 };
            
            // Limit output to available screen lines
            for (pid, proc) in matches.iter().take(lines_available.saturating_sub(1)) {
                let proc_name = proc.name().to_string_lossy();
                let mut output = format!("{} {} - {}: PID = {}, Name = {}", "bittytop:".bold().blue(), target.bold().green(), "Process".bold(), pid, proc_name.green());
                
                if show_cpu {
                    let cpu_usage = proc.cpu_usage();
                    output.push_str(&format!(", CPU = {} {:.2}%", get_bar(cpu_usage, bar_width), cpu_usage));
                }
                if show_mem {
                    let mem_pct = (proc.memory() as f32 / sys.total_memory() as f32) * 100.0;
                    output.push_str(&format!(", Mem = {} {:.2}%", get_bar(mem_pct, bar_width), mem_pct));
                }
                
                // Truncate line if it exceeds width
                println!("{}", truncate_ansi(&output, cols as usize));
            }
            
            if matches.len() > lines_available.saturating_sub(1) {
                println!("... and {} more processes", matches.len() - lines_available.saturating_sub(1));
            }
        }

        thread::sleep(Duration::from_millis(500));
    }
}
