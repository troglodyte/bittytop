use sysinfo::System;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use colored::*;
use crossterm::execute;
use crossterm::terminal::{Clear, ClearType};
use crossterm::cursor::MoveTo;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::time::Duration;
use std::io::{stdout, Write};

/// Provides an interactive fuzzy search interface to select a process or the entire system for monitoring.
/// Returns a vector containing the selected target (either a process name or "*" for system).
pub fn select_process(initial_query: Option<&str>) -> Vec<String> {
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
