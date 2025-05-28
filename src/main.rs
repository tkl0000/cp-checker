use cp_checker::{activate, inactivate, is_movement, load_cache, run, save_cache, update, Status};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::Text;
use ratatui::widgets::Block;
use ratatui::widgets::Paragraph;
use ratatui::Terminal;
use std::io;
use tui_textarea::{Input, Key, TextArea};

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

    load_cache(&mut textarea);

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
                footer = Paragraph::new(Text::from(format!("Executing {}", bin_path)))
                    .style(Style::default().fg(Color::DarkGray))
                    .block(Block::default());
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
                if which != 2 || is_movement(&input) {
                    textarea[which].input(input);
                }
                activate(&mut textarea[which], labels[which]);
                inactivate(&mut textarea[(which + 1) % 3], labels[(which + 1) % 3]);
                inactivate(&mut textarea[(which + 2) % 3], labels[(which + 2) % 3]);
                footer = Paragraph::new(Text::from(format!("Executing {}", bin_path)))
                    .style(Style::default().fg(Color::DarkGray))
                    .block(Block::default());
                save_cache(&textarea)?;
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
