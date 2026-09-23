use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use chrono::Local;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Position};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Terminal;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Clone, Debug)]
struct ChatMessage {
    timestamp: String,
    author: String,
    content: String,
    is_event: bool,
}

struct App {
    party_dir: PathBuf,
    messages_dir: PathBuf,
    user_log_path: PathBuf,
    username: String,
    input: String,
    messages: Vec<ChatMessage>,
    file_offsets: HashMap<PathBuf, u64>,
    scroll_offset: usize,
    should_quit: bool,
}

impl App {
    fn new() -> io::Result<Self> {
        let party_dir = std::env::var("PARTY_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp/terminal-party"));

        let messages_dir = party_dir.join("messages");
        fs::create_dir_all(&messages_dir).ok();

        #[cfg(unix)]
        {
            // Set sticky bit 1777 on messages directory if possible
            if let Ok(meta) = fs::metadata(&messages_dir) {
                let mut perms = meta.permissions();
                perms.set_mode(0o1777);
                let _ = fs::set_permissions(&messages_dir, perms);
            }
        }

        let username = get_current_username();
        let user_log_path = messages_dir.join(format!("{}.log", username));

        let mut app = Self {
            party_dir,
            messages_dir,
            user_log_path,
            username,
            input: String::new(),
            messages: Vec::new(),
            file_offsets: HashMap::new(),
            scroll_offset: 0,
            should_quit: false,
        };

        // Write join announcement to user's log
        app.log_event("joined the party");

        // Initial scan of all message logs
        app.poll_messages();

        Ok(app)
    }

    fn log_event(&self, event: &str) {
        self.append_to_user_log(&format!("*** {} ***", event));
    }

    fn log_message(&self, msg: &str) {
        self.append_to_user_log(msg);
    }

    fn append_to_user_log(&self, text: &str) {
        let time_str = Local::now().format("%H:%M:%S").to_string();
        let line = format!("[{}] {}\n", time_str, text);

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.user_log_path)
        {
            let _ = file.write_all(line.as_bytes());
            let _ = file.flush();

            #[cfg(unix)]
            {
                // Ensure the user's log is readable by all, but only writable by owner (0644)
                if let Ok(meta) = file.metadata() {
                    let mut perms = meta.permissions();
                    perms.set_mode(0o644);
                    let _ = fs::set_permissions(&self.user_log_path, perms);
                }
            }
        }
    }

    fn poll_messages(&mut self) {
        let Ok(entries) = fs::read_dir(&self.messages_dir) else {
            return;
        };

        let mut new_entries: Vec<ChatMessage> = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("log") {
                continue;
            }

            let Some(author) = path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string()) else {
                continue;
            };

            let Ok(mut file) = File::open(&path) else {
                continue;
            };

            let last_offset = self.file_offsets.entry(path.clone()).or_insert(0);

            if let Ok(metadata) = file.metadata() {
                if metadata.len() < *last_offset {
                    // File was truncated or rotated; reset offset
                    *last_offset = 0;
                }
            }

            if file.seek(SeekFrom::Start(*last_offset)).is_err() {
                continue;
            }

            let mut reader = BufReader::new(file);
            let mut line = String::new();

            while reader.read_line(&mut line).unwrap_or(0) > 0 {
                let trimmed = line.trim_end();
                if !trimmed.is_empty() {
                    if let Some(msg) = parse_log_line(trimmed, &author) {
                        new_entries.push(msg);
                    }
                }
                line.clear();
            }

            if let Ok(pos) = reader.stream_position() {
                *self.file_offsets.get_mut(&path).unwrap() = pos;
            }
        }

        if !new_entries.is_empty() {
            // Sort new entries by timestamp to keep chronological order
            new_entries.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
            self.messages.extend(new_entries);

            // Keep message buffer within a reasonable size
            if self.messages.len() > 1000 {
                let excess = self.messages.len() - 1000;
                self.messages.drain(0..excess);
            }
        }
    }

    fn handle_input(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
        let text = self.input.trim().to_string();
        self.input.clear();

        if text.is_empty() {
            return Ok(());
        }

        if text.starts_with('/') {
            let parts: Vec<&str> = text.split_whitespace().collect();
            let cmd = parts[0].to_lowercase();

            match cmd.as_str() {
                "/minecraft" | "/mc" => {
                    self.launch_minecraft(terminal)?;
                }
                "/help" => {
                    self.messages.push(ChatMessage {
                        timestamp: Local::now().format("%H:%M:%S").to_string(),
                        author: "SYSTEM".to_string(),
                        content: "Available commands: /minecraft (or /mc) - Play 3D Minecraft (Seed 7) | /credits - View credits | /clear - Clear feed | /quit - Exit".to_string(),
                        is_event: true,
                    });
                }
                "/credits" => {
                    self.messages.push(ChatMessage {
                        timestamp: Local::now().format("%H:%M:%S").to_string(),
                        author: "SYSTEM".to_string(),
                        content: "Created with ❤️ by Oliver (woliver99) | 3D Voxel Engine by TermCraft (vikvang/termcraft)".to_string(),
                        is_event: true,
                    });
                }
                "/clear" => {
                    self.messages.clear();
                }
                "/quit" | "/exit" => {
                    self.log_event("left the party");
                    self.should_quit = true;
                }
                _ => {
                    self.messages.push(ChatMessage {
                        timestamp: Local::now().format("%H:%M:%S").to_string(),
                        author: "SYSTEM".to_string(),
                        content: format!("Unknown command: '{}'. Type /help for available commands.", cmd),
                        is_event: true,
                    });
                }
            }
        } else {
            self.log_message(&text);
        }

        Ok(())
    }

    fn launch_minecraft(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
        // Announce joining Minecraft
        self.log_event("entered Minecraft");

        // Leave alternate screen & disable raw mode so termcraft has full terminal control
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen, crossterm::cursor::Show)?;

        // Locate termcraft binary
        let termcraft_bin = self.party_dir.join("termcraft");
        let bin_path = if termcraft_bin.exists() {
            termcraft_bin
        } else if Path::new("./termcraft").exists() {
            PathBuf::from("./termcraft")
        } else {
            PathBuf::from("termcraft")
        };

        // Run TermCraft with seed 7 and the player's authenticated username
        let _ = Command::new(&bin_path)
            .args(["--seed", "7", "--name", &self.username])
            .status();

        // Restore terminal sanity
        let _ = Command::new("stty").arg("sane").status();

        // Re-enter alternate screen & raw mode for chat TUI
        enable_raw_mode()?;
        execute!(terminal.backend_mut(), EnterAlternateScreen)?;
        terminal.clear()?;

        // Announce return to chat
        self.log_event("returned from Minecraft");

        Ok(())
    }
}

fn parse_log_line(line: &str, author: &str) -> Option<ChatMessage> {
    if !line.starts_with('[') {
        return None;
    }

    let end_bracket = line.find(']')?;
    let timestamp = line[1..end_bracket].to_string();
    let content = line[end_bracket + 1..].trim().to_string();

    let is_event = content.starts_with("***") && content.ends_with("***");

    Some(ChatMessage {
        timestamp,
        author: author.to_string(),
        content,
        is_event,
    })
}

fn get_current_username() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| {
            let output = Command::new("whoami").output();
            if let Ok(out) = output {
                String::from_utf8_lossy(&out.stdout).trim().to_string()
            } else {
                "player".to_string()
            }
        })
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        .take(20)
        .collect()
}

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
        terminal.draw(|f| ui(f, &app))?;

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

fn ui(f: &mut ratatui::Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(f.area());

    // 1. Header Banner with Credits
    let header_lines = vec![
        Line::from(vec![
            Span::styled("🎉 TERMINAL PARTY 🎉", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" - "),
            Span::styled("Multiplayer Terminal Chat & 3D Minecraft", Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("Created by: ", Style::default().fg(Color::DarkGray)),
            Span::styled("Oliver (woliver99)", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(" | 3D Engine: ", Style::default().fg(Color::DarkGray)),
            Span::styled("TermCraft (vikvang/termcraft)", Style::default().fg(Color::Magenta)),
        ]),
        Line::from(vec![
            Span::styled("Commands: ", Style::default().fg(Color::DarkGray)),
            Span::styled("/minecraft", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(" (or ", Style::default().fg(Color::DarkGray)),
            Span::styled("/mc", Style::default().fg(Color::Yellow)),
            Span::styled(") - Join Minecraft (Seed 7) | ", Style::default().fg(Color::DarkGray)),
            Span::styled("/help", Style::default().fg(Color::Cyan)),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled("/quit", Style::default().fg(Color::Red)),
        ]),
    ];

    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Party Central ")
        .title_alignment(Alignment::Center);

    let header = Paragraph::new(header_lines)
        .block(header_block)
        .alignment(Alignment::Center);

    f.render_widget(header, chunks[0]);

    // 2. Chat Feed
    let total_messages = app.messages.len();
    let visible_height = chunks[1].height.saturating_sub(2) as usize;

    let skip = if total_messages > visible_height + app.scroll_offset {
        total_messages - visible_height - app.scroll_offset
    } else {
        0
    };

    let items: Vec<ListItem> = app
        .messages
        .iter()
        .skip(skip)
        .take(visible_height)
        .map(|msg| {
            if msg.is_event {
                let content_clean = msg.content.trim_matches('*').trim();
                ListItem::new(Line::from(vec![
                    Span::styled(format!("[{}] ", msg.timestamp), Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("*** {} {} ***", msg.author, content_clean), Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC)),
                ]))
            } else {
                let author_color = match msg.author.as_str() {
                    "SYSTEM" => Color::LightRed,
                    name if name == app.username => Color::Cyan,
                    _ => Color::Green,
                };

                ListItem::new(Line::from(vec![
                    Span::styled(format!("[{}] ", msg.timestamp), Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("<{}> ", msg.author), Style::default().fg(author_color).add_modifier(Modifier::BOLD)),
                    Span::styled(&msg.content, Style::default().fg(Color::White)),
                ]))
            }
        })
        .collect();

    let title = if app.scroll_offset > 0 {
        format!(" Chat Feed (Scrolled +{}) - Press Esc to reset ", app.scroll_offset)
    } else {
        " Chat Feed ".to_string()
    };

    let messages_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(title);

    let list = List::new(items).block(messages_block);
    f.render_widget(list, chunks[1]);

    // 3. Input Line
    let prompt_prefix = format!("[{}] > ", app.username);
    let input_widget = Paragraph::new(Line::from(vec![
        Span::styled(&prompt_prefix, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(&app.input, Style::default().fg(Color::White)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Send Message or Command ")
    );

    f.render_widget(input_widget, chunks[2]);

    // Set cursor position at end of input line
    let cursor_x = chunks[2].x + 1 + prompt_prefix.len() as u16 + app.input.len() as u16;
    let cursor_y = chunks[2].y + 1;
    if cursor_x < chunks[2].x + chunks[2].width - 1 {
        f.set_cursor_position(Position::new(cursor_x, cursor_y));
    }
}
