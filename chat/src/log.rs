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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_log_line() {
        let msg = parse_log_line("[12:34:56] Hello world!", "alice").expect("valid line");
        assert_eq!(msg.timestamp, "12:34:56");
        assert_eq!(msg.author, "alice");
        assert_eq!(msg.content, "Hello world!");
        assert!(!msg.is_event);

        let event = parse_log_line("[12:35:00] *** joined the party ***", "bob").expect("valid line");
        assert_eq!(event.content, "*** joined the party ***");
        assert!(event.is_event);

        assert!(parse_log_line("invalid line", "charlie").is_none());
    }

    #[test]
    fn test_append_and_poll_messages() {
        let temp_dir = std::env::temp_dir().join(format!("chat-test-{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        let user_log = temp_dir.join("alice.log");
        append_user_log(&user_log, "first message");
        append_user_log(&user_log, "*** joined the party ***");

        let mut offsets = HashMap::new();
        let msgs = poll_new_messages(&temp_dir, &mut offsets);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].author, "alice");
        assert_eq!(msgs[0].content, "first message");
        assert!(msgs[1].is_event);

        // Polling again without new writes yields nothing
        let msgs2 = poll_new_messages(&temp_dir, &mut offsets);
        assert!(msgs2.is_empty());

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
