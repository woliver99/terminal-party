use ratatui::layout::{Alignment, Constraint, Direction, Layout, Position};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(f.area());

    // 1. Header Banner with Credits & Server Invite
    let invite_cmd = format!("{}/party.sh", app.party_dir.display());
    let header_lines = vec![
        Line::from(vec![
            Span::styled("TERMINAL PARTY", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("Created by: ", Style::default().fg(Color::DarkGray)),
            Span::styled("Oliver (woliver99)", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(" | Terminal Minecraft: ", Style::default().fg(Color::DarkGray)),
            Span::styled("TermCraft (vikvang/termcraft)", Style::default().fg(Color::Magenta)),
        ]),
        Line::from(vec![
            Span::styled("Invite others on this server: ", Style::default().fg(Color::DarkGray)),
            Span::styled(invite_cmd, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(" (or type /invite)", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("Commands: ", Style::default().fg(Color::DarkGray)),
            Span::styled("/minecraft", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(" (or ", Style::default().fg(Color::DarkGray)),
            Span::styled("/mc", Style::default().fg(Color::Yellow)),
            Span::styled(") - Join Minecraft | ", Style::default().fg(Color::DarkGray)),
            Span::styled("/invite", Style::default().fg(Color::Cyan)),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled("/help", Style::default().fg(Color::Cyan)),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled("/quit", Style::default().fg(Color::Red)),
        ]),
    ];

    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title("Info")
        .title_alignment(Alignment::Left);

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
