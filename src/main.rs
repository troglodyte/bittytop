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

fn monitor_process(target: &str) {
    let mut sys = System::new_all();
    let machine = Machine::new();
    
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
                match key.code {
                    KeyCode::Char('c') => show_cpu = !show_cpu,
                    KeyCode::Char('m') => show_mem = !show_mem,
                    KeyCode::Char('g') => show_gpu = !show_gpu,
                    KeyCode::Char('q') => break,
                    _ => {}
                }
            }
        }

        // Refresh necessary components
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        sys.refresh_memory();
        let gpu = machine.graphics_status(); 

        let mut found = false;
        
        // Print header
        print!("{}[2J{}[1;1H", 27 as char, 27 as char); // Clear screen
        if show_gpu {
            if let Some(g) = gpu.first() {
                println!("{} GPU: {} {}% | Temp: {}°C | Mem: {}GB", "bittytop:".bold().blue(), get_bar(g.gpu as f32), g.gpu, g.temperature, g.memory_used / 1024 / 1024 / 1024);
            } else {
                 println!("{} GPU: N/A", "bittytop:".bold().blue());
            }
            println!();
        }

        let total_mem = sys.total_memory() as f32;
        for (pid, proc) in sys.processes() {
            let proc_name = proc.name().to_string_lossy();
            let pid_str = pid.to_string();
            
            if proc_name.to_lowercase().contains(&target.to_lowercase()) || pid_str == target {
                let mut output = format!("{} {} - {}: PID = {}, Name = {}", "bittytop:".bold().blue(), target.bold().green(), "Process".bold(), pid, proc_name.green());
                
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
            println!("{} {} - {}", "bittytop:".bold().blue(), target.bold().green(), "Process not found or exited.".red());
        }

        thread::sleep(Duration::from_millis(500));
    }
}
