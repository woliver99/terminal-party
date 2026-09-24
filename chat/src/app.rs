use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::Local;
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::log::{append_user_log, poll_new_messages, ChatMessage};

pub struct App {
    pub party_dir: PathBuf,
    pub messages_dir: PathBuf,
    pub user_log_path: PathBuf,
    pub username: String,
    pub input: String,
    pub messages: Vec<ChatMessage>,
    pub file_offsets: HashMap<PathBuf, u64>,
    pub scroll_offset: usize,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> io::Result<Self> {
        let party_dir = std::env::var("PARTY_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp/terminal-party"));

        let messages_dir = if party_dir.join("data").join("messages").exists()
            || party_dir.join("data").exists() {
            party_dir.join("data").join("messages")
        } else if party_dir.join("messages").exists() {
            party_dir.join("messages")
        } else {
            party_dir.join("data").join("messages")
        };
        fs::create_dir_all(&messages_dir).ok();

        #[cfg(unix)]
        {
            // Ensure data directory and messages directory have sticky bit (1777)
            if let Some(parent) = messages_dir.parent() {
                if let Ok(meta) = fs::metadata(parent) {
                    let mut perms = meta.permissions();
                    perms.set_mode(0o1777);
                    let _ = fs::set_permissions(parent, perms);
                }
            }
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

        // Announce joining the party
        app.log_event("joined the party");

        // Initial scan of existing logs
        app.poll_messages();

        Ok(app)
    }

    pub fn log_event(&self, event: &str) {
        append_user_log(&self.user_log_path, &format!("*** {} ***", event));
    }

    pub fn log_message(&self, msg: &str) {
        append_user_log(&self.user_log_path, msg);
    }

    pub fn poll_messages(&mut self) {
        let new_entries = poll_new_messages(&self.messages_dir, &mut self.file_offsets);

        if !new_entries.is_empty() {
            self.messages.extend(new_entries);

            // Cap in-memory history to the last 1000 messages
            if self.messages.len() > 1000 {
                let excess = self.messages.len() - 1000;
                self.messages.drain(0..excess);
            }
        }
    }

    pub fn handle_input(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
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
                    let creative = parts.get(1).map(|s| s.eq_ignore_ascii_case("creative") || *s == "-c").unwrap_or(false);
                    self.launch_minecraft(terminal, creative)?;
                }
                "/creative" => {
                    self.launch_minecraft(terminal, true)?;
                }
                "/diag" | "/diagnostics" => {
                    self.launch_diagnostics(terminal)?;
                }
                "/update" => {
                    let local_cmd = format!("{}/party.sh --update", self.party_dir.display());
                    self.messages.push(ChatMessage {
                        timestamp: Local::now().format("%H:%M:%S").to_string(),
                        author: "SYSTEM".to_string(),
                        content: format!("To update Terminal Party to the latest release, exit the party (/quit) and run: {}", local_cmd),
                        is_event: true,
                    });
                }
                "/help" => {
                    self.messages.push(ChatMessage {
                        timestamp: Local::now().format("%H:%M:%S").to_string(),
                        author: "SYSTEM".to_string(),
                        content: "Available commands: /minecraft (or /mc) [creative] - Play 3D Minecraft | /creative - Play in creative mode | /diag - Keyboard diagnostics | /update - Update instructions | /invite - Show invite command | /credits - View credits | /clear - Clear feed | /quit - Exit".to_string(),
                        is_event: true,
                    });
                }
                "/invite" | "/share" => {
                    let local_cmd = format!("{}/party.sh", self.party_dir.display());
                    self.messages.push(ChatMessage {
                        timestamp: Local::now().format("%H:%M:%S").to_string(),
                        author: "SYSTEM".to_string(),
                        content: format!("To invite others on this SSH server, have them run: {}", local_cmd),
                        is_event: true,
                    });
                    self.messages.push(ChatMessage {
                        timestamp: Local::now().format("%H:%M:%S").to_string(),
                        author: "SYSTEM".to_string(),
                        content: "Or install on a new machine: bash -c \"$(curl -fsSL https://raw.githubusercontent.com/woliver99/terminal-party/main/party.sh)\"".to_string(),
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

    pub fn launch_minecraft(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, creative: bool) -> io::Result<()> {
        if creative {
            self.log_event("entered Minecraft (Creative)");
        } else {
            self.log_event("entered Minecraft");
        }

        // Leave alternate screen & raw mode so TermCraft has full terminal access
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen, crossterm::cursor::Show)?;

        let termcraft_bin = self.party_dir.join("termcraft");
        let bin_path = if termcraft_bin.exists() {
            termcraft_bin
        } else if Path::new("./termcraft").exists() {
            PathBuf::from("./termcraft")
        } else {
            PathBuf::from("termcraft")
        };

        // Run TermCraft with seed 7, player name, and optional creative mode
        let mut cmd = Command::new(&bin_path);
        cmd.args(["--seed", "7", "--name", &self.username]);
        if creative {
            cmd.arg("--creative");
        }
        cmd.env("PARTY_DIR", &self.party_dir);
        let _ = cmd.status();

        // Restore terminal state
        let _ = Command::new("stty").arg("sane").status();

        // Re-enter alternate screen & raw mode for chat TUI
        enable_raw_mode()?;
        execute!(terminal.backend_mut(), EnterAlternateScreen)?;
        terminal.clear()?;

        self.log_event("returned from Minecraft");

        Ok(())
    }

    pub fn launch_diagnostics(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen, crossterm::cursor::Show)?;

        let termcraft_bin = self.party_dir.join("termcraft");
        let bin_path = if termcraft_bin.exists() {
            termcraft_bin
        } else if Path::new("./termcraft").exists() {
            PathBuf::from("./termcraft")
        } else {
            PathBuf::from("termcraft")
        };

        let mut cmd = Command::new(&bin_path);
        cmd.arg("--diag");
        cmd.env("PARTY_DIR", &self.party_dir);
        let _ = cmd.status();

        println!("\r\nPress Enter to return to chat...");
        let mut line = String::new();
        let _ = io::stdin().read_line(&mut line);

        let _ = Command::new("stty").arg("sane").status();
        enable_raw_mode()?;
        execute!(terminal.backend_mut(), EnterAlternateScreen)?;
        terminal.clear()?;
        Ok(())
    }
}

pub fn get_current_username() -> String {
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
