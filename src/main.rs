use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Text;
use ratatui::widgets::Paragraph;
use ratatui::widgets::{Block, Borders};
use ratatui::Terminal;
use std::io::{self, Read, Write};
use std::process::{Command, Stdio};
use std::time::Duration;
use std::time::Instant;
use tui_textarea::{Input, Key, TextArea};
use wait_timeout::ChildExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Pass,
    Fail,
    Error,
    Idle,
}

fn update(textarea: &mut TextArea<'_>, label: &'static str, status: Status) {
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

fn inactivate(textarea: &mut TextArea<'_>, label: &'static str) {
    textarea.set_cursor_line_style(Style::default());
    textarea.set_cursor_style(Style::default());
    textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::DarkGray))
            .title(label),
    );
}

fn activate(textarea: &mut TextArea<'_>, label: &'static str) {
    textarea.set_cursor_line_style(Style::default().add_modifier(Modifier::UNDERLINED));
    textarea.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
    textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default())
            .title(label),
    );
}

fn run(textareas: &mut [TextArea], bin_path: &str) -> (Status, String) {
    let input = textareas[0].lines().join("\n");
    if input.len() == 0 {
        return (Status::Error, String::from("ERR: Empty Input"));
    }

    let mut result: Vec<String> = Vec::from([]);
    let mut duration = 0;
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
                    duration = start.elapsed().as_millis();
                    let output = child.wait_with_output().unwrap();
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    let output_lines: Vec<String> = output_str.lines().map(String::from).collect();
                    result = output_lines;
                }
                None => {
                    child.kill().unwrap();
                    let output = child.wait_with_output().expect("Failed to wait");
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    let output_lines: Vec<String> = output_str.lines().map(String::from).collect();
                    while textareas[2].cursor() != (0, 0) {
                        textareas[2].delete_line_by_head();
                    }
                    for line in output_lines.iter() {
                        textareas[2].insert_str(line);
                        textareas[2].insert_newline();
                    }

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

    let lines = result;
    while textareas[2].cursor() != (0, 0) {
        textareas[2].delete_line_by_head();
    }
    for line in lines.iter() {
        textareas[2].insert_str(line);
        textareas[2].insert_newline();
    }
    let output = lines.join("\n");
    let expected_output = textareas[1].lines().join("\n");
    if output.trim().trim_end_matches("\n") == expected_output.trim().trim_end_matches("\n") {
        return (Status::Pass, String::from(format!("AC | {} ms", duration)));
    } else {
        return (Status::Fail, String::from(format!("WA | {} ms", duration)));
    }
}

fn main() -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    enable_raw_mode()?;
    crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let Some(bin_path) = std::env::args().nth(1) else {
        disable_raw_mode()?;
        crossterm::execute!(
            term.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        term.show_cursor()?;
        eprintln!("Usage: cp-checker <path-to-binary>");
        return Ok(());
    };

    let mut textarea = [
        TextArea::default(),
        TextArea::default(),
        TextArea::default(),
    ];
    let mut footer = Paragraph::new(Text::from(format!(
        "Ctrl+R = run | Ctrl+X = switch | Esc = quit \nExecuting {}",
        bin_path
    )))
    .style(Style::default().fg(Color::DarkGray))
    .block(Block::default());
    let labels = ["Input", "Expected Output", "Output"];

    let mut which = 0;
    activate(&mut textarea[0], labels[0]);
    inactivate(&mut textarea[1], labels[1]);
    inactivate(&mut textarea[2], labels[2]);

    cp_checker::load_cache(&mut textarea);

    loop {
        term.draw(|f| {
            let main_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(90), Constraint::Percentage(10)])
                .split(f.area());
            let top_row = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(33),
                    Constraint::Percentage(33),
                    Constraint::Percentage(34),
                ])
                .split(main_layout[0]);
            for (textarea, chunk) in textarea.iter().zip(top_row.iter()) {
                f.render_widget(textarea, *chunk);
            }
            f.render_widget(&footer, main_layout[1]);
        })?;
        match crossterm::event::read()?.into() {
            Input { key: Key::Esc, .. } => break,
            Input {
                key: Key::Char('x'),
                ctrl: true,
                ..
            } => {
                which = (which + 1) % 3;
                activate(&mut textarea[which], labels[which]);
                inactivate(&mut textarea[(which + 1) % 3], labels[(which + 1) % 3]);
                inactivate(&mut textarea[(which + 2) % 3], labels[(which + 2) % 3]);
            }
            Input {
                key: Key::Char('r'),
                ctrl: true,
                ..
            } => {
                let res = run(&mut textarea, &bin_path);
                let res_code = res.0;
                let res_message = res.1;
                if res_code == Status::Pass {
                    update(&mut textarea[0], labels[0], Status::Pass);
                    update(&mut textarea[1], labels[1], Status::Pass);
                    update(&mut textarea[2], labels[2], Status::Pass);
                    footer = Paragraph::new(Text::from(res_message))
                        .style(Style::default().fg(Color::Green))
                        .block(Block::default());
                } else if res_code == Status::Fail {
                    update(&mut textarea[0], labels[0], Status::Fail);
                    update(&mut textarea[1], labels[1], Status::Fail);
                    update(&mut textarea[2], labels[2], Status::Fail);
                    footer = Paragraph::new(Text::from(res_message))
                        .style(Style::default().fg(Color::Red))
                        .block(Block::default());
                } else if res_code == Status::Error {
                    update(&mut textarea[0], labels[0], Status::Error);
                    update(&mut textarea[1], labels[1], Status::Error);
                    update(&mut textarea[2], labels[2], Status::Error);
                    footer = Paragraph::new(Text::from(res_message))
                        .style(Style::default().fg(Color::Yellow))
                        .block(Block::default());
                }
            }
            input @ Input { .. } => {
                if which != 2 || cp_checker::is_movement(&input) {
                    textarea[which].input(input);
                }
                activate(&mut textarea[which], labels[which]);
                inactivate(&mut textarea[(which + 1) % 3], labels[(which + 1) % 3]);
                inactivate(&mut textarea[(which + 2) % 3], labels[(which + 2) % 3]);
                footer = Paragraph::new(Text::from(format!("Executing {}", bin_path)))
                    .style(Style::default().fg(Color::DarkGray))
                    .block(Block::default());
                cp_checker::save_cache(&textarea)?;
            } // input => {}
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(
        term.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    term.show_cursor()?;
    Ok(())
}
