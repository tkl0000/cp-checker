use ratatui::{
    style::{Color, Modifier, Style},
    widgets::{Block, Borders},
};
use serde::{Deserialize, Serialize};
use std::{fs::File, process::Command, time::Instant};
use std::{io::Write, process::Stdio};
use std::{path::PathBuf, time::Duration};
use tui_textarea::{Input, Key, TextArea};
use wait_timeout::ChildExt;

pub fn insert_lines(textarea: &mut TextArea<'_>, lines: &Vec<String>) {
    while textarea.cursor() != (0, 0) {
        textarea.delete_line_by_head();
    }
    for line in lines.iter() {
        textarea.insert_str(line);
        textarea.insert_newline();
    }
}

pub fn is_movement(input: &Input) -> bool {
    matches!(
        input.key,
        Key::Up
            | Key::Down
            | Key::Left
            | Key::Right
            | Key::Home
            | Key::End
            | Key::PageUp
            | Key::PageDown
            | Key::MouseScrollDown
            | Key::MouseScrollUp
    )
}

pub fn get_cache_path() -> PathBuf {
    let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(".cp-checker-cache.json");
    path
}

#[derive(Serialize, Deserialize)]
struct CachedInputs {
    input: Vec<String>,
    expected: Vec<String>,
}

pub fn save_cache(textareas: &[TextArea]) -> std::io::Result<()> {
    let cache = CachedInputs {
        input: Vec::from(textareas[0].lines()),
        expected: Vec::from(textareas[1].lines()),
    };

    let path = get_cache_path();
    let json = serde_json::to_string(&cache)?;
    let mut file = File::create(path)?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

pub fn load_cache(textareas: &mut [TextArea]) {
    let path = get_cache_path();
    if let Ok(data) = std::fs::read_to_string(path) {
        if let Ok(cache) = serde_json::from_str::<CachedInputs>(&data) {
            insert_lines(&mut textareas[0], &cache.input);
            insert_lines(&mut textareas[1], &cache.expected);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Pass,
    Fail,
    Error,
    Idle,
}

pub fn update(textarea: &mut TextArea<'_>, label: &'static str, status: Status) {
    textarea.set_cursor_line_style(Style::default());
    textarea.set_cursor_style(Style::default());
    textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(match status {
                Status::Pass => Color::Green,
                Status::Fail => Color::Red,
                Status::Idle => Color::DarkGray,
                Status::Error => Color::Yellow,
            }))
            .title(label),
    );
}

pub fn inactivate(textarea: &mut TextArea<'_>, label: &'static str) {
    textarea.set_cursor_line_style(Style::default());
    textarea.set_cursor_style(Style::default());
    textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::DarkGray))
            .title(label),
    );
}

pub fn activate(textarea: &mut TextArea<'_>, label: &'static str) {
    textarea.set_cursor_line_style(Style::default().add_modifier(Modifier::UNDERLINED));
    textarea.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
    textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default())
            .title(label),
    );
}

pub fn run(textareas: &mut [TextArea], bin_path: &str) -> (Status, String) {
    let input = textareas[0].lines().join("\n");
    if input.len() == 0 {
        return (Status::Error, String::from("ERR: Empty Input"));
    }

    match Command::new(&bin_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(mut child) => {
            if let Some(stdin) = child.stdin.as_mut() {
                stdin
                    .write_all(input.as_bytes())
                    .expect("Failed to write to stdin");
            }
            let ms = 2000;
            let timeout_ms = Duration::from_millis(ms);
            let start = Instant::now();
            match child.wait_timeout(timeout_ms).unwrap() {
                Some(_status) => {
                    let duration = start.elapsed().as_millis();
                    let output = child.wait_with_output().unwrap();
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    let output_lines: Vec<String> = output_str.lines().map(String::from).collect();
                    insert_lines(&mut textareas[2], &output_lines);
                    let output = output_lines.join("\n");
                    let expected_output = textareas[1].lines().join("\n");
                    if output.trim().trim_end_matches("\n")
                        == expected_output.trim().trim_end_matches("\n")
                    {
                        return (Status::Pass, String::from(format!("AC | {} ms", duration)));
                    } else {
                        return (Status::Fail, String::from(format!("WA | {} ms", duration)));
                    }
                }
                None => {
                    child.kill().unwrap();
                    let output = child.wait_with_output().expect("Failed to wait");
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    let output_lines: Vec<String> = output_str.lines().map(String::from).collect();
                    insert_lines(&mut textareas[2], &output_lines);

                    return (
                        Status::Error,
                        String::from(format!("ERR: Time Limit Exceeded ({} ms)", ms)),
                    );
                }
            };
        }
        Err(_e) => {
            return (
                Status::Error,
                String::from(format!("ERR: Failed to execute {}", bin_path)),
            );
        }
    }
}

#[cfg(test)]
mod tests {}
