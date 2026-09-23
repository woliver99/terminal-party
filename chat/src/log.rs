use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use chrono::Local;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub timestamp: String,
    pub author: String,
    pub content: String,
    pub is_event: bool,
}

/// Parses a line formatted as `[HH:MM:SS] content` attributed to a specific author.
pub fn parse_log_line(line: &str, author: &str) -> Option<ChatMessage> {
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

/// Appends a timestamped line to the user's personal log file (mode 0644).
pub fn append_user_log(user_log_path: &Path, text: &str) {
    let time_str = Local::now().format("%H:%M:%S").to_string();
    let line = format!("[{}] {}\n", time_str, text);

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(user_log_path)
    {
        let _ = file.write_all(line.as_bytes());
        let _ = file.flush();

        #[cfg(unix)]
        {
            // Readable by all, writable only by owner (0644) to prevent spoofing
            if let Ok(meta) = file.metadata() {
                let mut perms = meta.permissions();
                perms.set_mode(0o644);
                let _ = fs::set_permissions(user_log_path, perms);
            }
        }
    }
}

/// Scans the messages directory for new lines in any `*.log` file.
pub fn poll_new_messages(
    messages_dir: &Path,
    file_offsets: &mut HashMap<PathBuf, u64>,
) -> Vec<ChatMessage> {
    let Ok(entries) = fs::read_dir(messages_dir) else {
        return Vec::new();
    };

    let mut new_entries = Vec::new();

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

        let last_offset = file_offsets.entry(path.clone()).or_insert(0);

        if let Ok(metadata) = file.metadata() {
            if metadata.len() < *last_offset {
                // File was rotated or truncated; reset offset
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
            *file_offsets.get_mut(&path).unwrap() = pos;
        }
    }

    // Sort new entries chronologically
    new_entries.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    new_entries
}
