use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders};
use ratatui::Terminal;
use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::cmp;
use tui_textarea::{Input, Key, TextArea};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
   Pass,
   Fail,
   Idle
}

fn update(textarea: &mut TextArea<'_>, label: &'static str, status: Status) {
    textarea.set_cursor_line_style(Style::default());
    textarea.set_cursor_style(Style::default());
    textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(
                    match status {
                        Status::Pass => { Color::Green },
                        Status::Fail => { Color::Red }, 
                        Status::Idle => { Color::DarkGray },
                    } 
            ))
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

fn run(textareas: &mut [TextArea], bin_path: &str) -> bool {
    let input = textareas[0].lines().join("\n");
    let result = Command::new(&bin_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(input.as_bytes())?;
            }
            let output = child.wait_with_output()?;
            let output_str = String::from_utf8_lossy(&output.stdout);
            let output_lines: Vec<String> = output_str.lines().map(String::from).collect();
            Ok(output_lines)
        });
    let mut pass = true;
    if let Ok(lines) = result {
        while textareas[2].cursor() != (0, 0) {
            textareas[2].delete_line_by_head();
        }
        for line in lines.iter() {
            textareas[2].insert_str(line);
            textareas[2].insert_newline();
        }
        let expected = textareas[1].lines();
        let lines_len = lines.len();
        let expected_len = expected.len();
        let mut pnt = 0; 
        if lines_len != expected_len {
            pass = false;
        }
        else {
            while pnt < cmp::min(lines_len, expected_len) {
                if &lines[pnt] != &expected[pnt] {
                    pass = false;
                } 
                pnt += 1;
            }
        }
    }
    else {
        textareas[2].insert_str("Failed to execute binary".to_string());
    }
    pass
}

fn main() -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    enable_raw_mode()?;
    crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let mut textarea = [TextArea::default(), TextArea::default(), TextArea::default()];
    let labels = ["Input", "Expected Output", "Output"];

    let mut args = std::env::args();
    args.next();
    let bin_path = args.next().expect("Usage: program <path-to-binary>");

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(33), Constraint::Percentage(33), Constraint::Percentage(33)].as_ref());

    let mut which = 0;
    activate(&mut textarea[0], labels[0]);
    inactivate(&mut textarea[1], labels[1]);
    inactivate(&mut textarea[2], labels[2]);

    loop {
        term.draw(|f| {
            let chunks = layout.split(f.area());
            for (textarea, chunk) in textarea.iter().zip(chunks.iter()) {
                f.render_widget(textarea, *chunk);
            }
        })?;
        match crossterm::event::read()?.into() {
            Input { key: Key::Esc, .. } => break,
            Input {
                key: Key::Char('x'),
                ctrl: true,
                ..
            } => {
                inactivate(&mut textarea[which], labels[which]);
                which = (which + 1) % 2;
                activate(&mut textarea[which], labels[which]);
                update(&mut textarea[2], labels[2], Status::Idle);
            },
            Input {
                key: Key::Char('r'),
                ctrl: true,
                ..
            } => {
                let res = run(&mut textarea, &bin_path);
                if res {
                    update(&mut textarea[0], labels[0], Status::Pass);
                    update(&mut textarea[1], labels[1], Status::Pass);
                    update(&mut textarea[2], "Pass", Status::Pass);
                }
                else {
                    update(&mut textarea[0], labels[0], Status::Fail);
                    update(&mut textarea[1], labels[1], Status::Fail);
                    update(&mut textarea[2], "Fail", Status::Fail);
                }
            },
            input => {
                textarea[which].input(input);
                activate(&mut textarea[which], labels[which]);
                inactivate(&mut textarea[(which + 1) % 2], labels[(which + 1) % 2]);
                update(&mut textarea[2], labels[2], Status::Idle);
            }
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(
        term.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    term.show_cursor()?;

    // println!("Left textarea: {:?}", textarea[0].lines());
    // println!("Right textarea: {:?}", textarea[1].lines());
    Ok(())
}
