use std::thread;
use std::time::Duration;
use std::io::{stdout, Write};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};

use crate::service::MonitorService;
use crate::view::prepare_view;

/// The main monitoring loop. It periodically collects system data, handles user input for toggling
/// metrics, and updates the terminal display.
pub fn monitor_process(targets: Vec<String>, show_net: bool) {
    let mut service = MonitorService::new();
    let num_cpus = service.get_num_cpus();

    // Initial wait to ensure accurate first measurements if needed, 
    // although MonitorService::new already does a refresh_all.
    thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    
    let mut order = if show_net { vec!["net"] } else { vec!["cpu", "mem"] };
    let mut sort_ascending = false;

    'main_loop: loop {
        // Handle input - process all pending events
        while event::poll(Duration::from_millis(0)).unwrap() {
            if let Event::Key(key) = event::read().unwrap()
                && key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('a') => {
                            sort_ascending = true;
                        }
                        KeyCode::Char('d') => {
                            sort_ascending = false;
                        }
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

        let data = service.tick();
        let buf = prepare_view(&data, &targets, &order, num_cpus, sort_ascending);
        
        stdout().write_all(&buf).unwrap();
        stdout().flush().unwrap();
        thread::sleep(Duration::from_millis(1000));
    }
}
