use std::env;
use std::thread;
use std::time::Duration;
use colored::*;
use sysinfo::System;
use machine_info::Machine;
use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <pid_or_name>", args[0].bold());
        return;
    }

    let target = &args[1];
    
    enable_raw_mode().unwrap();
    monitor_process(target);
    disable_raw_mode().unwrap();
}

fn get_bar(percentage: f32, width: usize) -> String {
    let filled = (percentage.clamp(0.0, 100.0) / 100.0 * width as f32).round() as usize;
    let empty = width - filled;
    format!("[{}{}]", "█".repeat(filled).cyan(), "░".repeat(empty))
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

        let mut found = false;
        
        // Print header
        print!("{}[2J{}[1;1H", 27 as char, 27 as char); // Clear screen
        if show_gpu {
            if let Some(g) = gpu.first() {
                println!("{} GPU: {} {}% | Temp: {}°C | Mem: {}GB", "bittytop:".bold().blue(), get_bar(g.gpu as f32, 10), g.gpu, g.temperature, g.memory_used / 1024 / 1024 / 1024);
            } else {
                 println!("{} GPU: N/A", "bittytop:".bold().blue());
            }
            println!();
        }

        for (pid, proc) in sys.processes() {
            let proc_name = proc.name().to_string_lossy();
            let pid_str = pid.to_string();
            
            if proc_name.to_lowercase().contains(&target.to_lowercase()) || pid_str == target {
                let mut output = format!("{} {} - {}: PID = {}, Name = {}", "bittytop:".bold().blue(), target.bold().green(), "Process".bold(), pid, proc_name.green());
                
                if show_cpu {
                    let cpu_usage = proc.cpu_usage();
                    output.push_str(&format!(", CPU = {} {:.2}%", get_bar(cpu_usage, 10), cpu_usage));
                }
                if show_mem {
                    let mem_pct = (proc.memory() as f32 / sys.total_memory() as f32) * 100.0;
                    output.push_str(&format!(", Mem = {} {:.2}%", get_bar(mem_pct, 10), mem_pct));
                }
                
                println!("{}", output);
                found = true;
            }
        }

        if !found {
            println!("{} {} - {}", "bittytop:".bold().blue(), target.bold().green(), "Process not found or exited.".red());
        }

        thread::sleep(Duration::from_millis(500));
    }
}
