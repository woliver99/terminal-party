mod app;
mod log;
mod ui;

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::app::App;

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal);

    // Teardown terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, crossterm::cursor::Show)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Party Chat exited with error: {:?}", err);
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut app = App::new()?;
    let tick_rate = Duration::from_millis(150);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui::render(f, &app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_millis(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.log_event("left the party");
                        return Ok(());
                    }
                    KeyCode::Char(c) => {
                        app.input.push(c);
                    }
                    KeyCode::Backspace => {
                        app.input.pop();
                    }
                    KeyCode::Enter => {
                        app.handle_input(terminal)?;
                        if app.should_quit {
                            return Ok(());
                        }
                    }
                    KeyCode::Up => {
                        app.scroll_offset = app.scroll_offset.saturating_add(1);
                    }
                    KeyCode::Down => {
                        app.scroll_offset = app.scroll_offset.saturating_sub(1);
                    }
                    KeyCode::PageUp => {
                        app.scroll_offset = app.scroll_offset.saturating_add(10);
                    }
                    KeyCode::PageDown => {
                        app.scroll_offset = app.scroll_offset.saturating_sub(10);
                    }
                    KeyCode::Esc => {
                        app.scroll_offset = 0;
                    }
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.poll_messages();
            last_tick = Instant::now();
        }
    }
}
