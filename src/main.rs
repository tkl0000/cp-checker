use crossterm::event::{DisableMouseCapture, EnableMouseCapture, KeyEvent};
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
use std::cmp;
use std::io::{self, Write};
use std::process::{Command, Stdio};
use tui_textarea::{Input, Key, TextArea};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Pass,
    Fail,
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

fn run(textareas: &mut [TextArea], bin_path: &str) -> bool {
    let input = textareas[0].lines().join("\n");
    if input.len() == 0 {
        return false;
    }
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
        } else {
            while pnt < cmp::min(lines_len, expected_len) {
                if &lines[pnt] != &expected[pnt] {
                    pass = false;
                }
                pnt += 1;
            }
        }
    } else {
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

    let mut textarea = [
        TextArea::default(),
        TextArea::default(),
        TextArea::default(),
    ];
    let mut footer = Paragraph::new(Text::from("Ctrl+R = run | Ctrl+X = switch | Esc = quit"))
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default());
    let labels = ["Input", "Expected Output", "Output"];

    let mut args = std::env::args();
    args.next();
    let bin_path = args.next().expect("Usage: program <path-to-binary>");

    let mut which = 0;
    activate(&mut textarea[0], labels[0]);
    inactivate(&mut textarea[1], labels[1]);
    inactivate(&mut textarea[2], labels[2]);

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
                inactivate(&mut textarea[which], labels[which]);
                which = (which + 1) % 2;
                activate(&mut textarea[which], labels[which]);
                update(&mut textarea[2], labels[2], Status::Idle);
            }
            Input {
                key: Key::Char('r'),
                ctrl: true,
                ..
            } => {
                let res = run(&mut textarea, &bin_path);
                if res {
                    update(&mut textarea[0], labels[0], Status::Pass);
                    update(&mut textarea[1], labels[1], Status::Pass);
                    update(&mut textarea[2], labels[2], Status::Pass);
                    footer = Paragraph::new(Text::from("PASS"))
                        .style(Style::default().fg(Color::Green))
                        .block(Block::default());
                } else {
                    update(&mut textarea[0], labels[0], Status::Fail);
                    update(&mut textarea[1], labels[1], Status::Fail);
                    update(&mut textarea[2], labels[2], Status::Fail);
                    footer = Paragraph::new(Text::from("FAIL"))
                        .style(Style::default().fg(Color::Red))
                        .block(Block::default());
                }
            }
            input @ Input { .. } => {
                textarea[which].input(input);
                activate(&mut textarea[which], labels[which]);
                inactivate(&mut textarea[(which + 1) % 2], labels[(which + 1) % 2]);
                update(&mut textarea[2], labels[2], Status::Idle);
                footer = Paragraph::new(Text::from("Ctrl+R = run | Ctrl+X = switch | Esc = quit"))
                    .style(Style::default().fg(Color::DarkGray))
                    .block(Block::default());
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

    // println!("Left textarea: {:?}", textarea[0].lines());
    // println!("Right textarea: {:?}", textarea[1].lines());
    Ok(())
}
