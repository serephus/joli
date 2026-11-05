use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::app::{App, PREVIEW_LIMIT, StatusKind};

pub fn draw(frame: &mut Frame, app: &App) {
    let [
        header_area,
        preview_area,
        input_area,
        status_area,
        help_area,
    ] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(5),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    draw_header(frame, header_area, app);
    draw_preview(frame, preview_area, app);
    draw_input(frame, input_area, app);
    draw_status(frame, status_area, app);
    draw_help(frame, help_area);
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let path = display_safe(&app.document.path.display().to_string());
    let mut title = vec![
        Span::styled(
            "joli",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(path, Style::default().add_modifier(Modifier::BOLD)),
    ];
    if app.dirty {
        title.push(Span::styled(
            "  [unsaved]",
            Style::default().fg(Color::Yellow),
        ));
    }

    let match_state = match &app.search {
        Some(search) => {
            let plural = if search.matches.len() == 1 { "" } else { "s" };
            format!("{} matching line{plural}", search.matches.len())
        }
        None => "no active search".to_string(),
    };
    let (save_state, save_color) = if app.dirty {
        ("unsaved changes".to_string(), Color::Yellow)
    } else if app.saved_to.is_some() {
        ("saved".to_string(), Color::Green)
    } else {
        ("not saved yet".to_string(), Color::DarkGray)
    };

    let details = Line::from(vec![
        Span::styled(
            format!("{} lines", app.document.lines.len()),
            Style::default().fg(Color::Gray),
        ),
        Span::styled("  ·  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{} deleted", app.document.deleted),
            Style::default().fg(Color::Gray),
        ),
        Span::styled("  ·  ", Style::default().fg(Color::DarkGray)),
        Span::styled(match_state, Style::default().fg(Color::Gray)),
        Span::styled("  ·  ", Style::default().fg(Color::DarkGray)),
        Span::styled(save_state, Style::default().fg(save_color)),
    ]);

    frame.render_widget(Paragraph::new(vec![Line::from(title), details]), area);
}

fn draw_preview(frame: &mut Frame, area: Rect, app: &App) {
    let preview_limit = PREVIEW_LIMIT.min(area.height.saturating_sub(2) as usize);
    let (title, title_color, border_color) = match &app.search {
        Some(search) if search.matches.is_empty() => (
            " No matching lines ".to_string(),
            Color::Red,
            Color::DarkGray,
        ),
        Some(search) => {
            let shown = search.matches.len().min(preview_limit);
            let plural = if search.matches.len() == 1 { "" } else { "s" };
            (
                format!(
                    " {} matching line{plural}  ·  showing {shown} ",
                    search.matches.len()
                ),
                Color::Yellow,
                Color::Yellow,
            )
        }
        None => (" Preview ".to_string(), Color::Gray, Color::DarkGray),
    };

    let mut block = Block::bordered()
        .border_style(Style::default().fg(border_color))
        .title(Line::from(Span::styled(
            title,
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
        )));

    if let Some(search) = &app.search {
        block = block.title_bottom(Line::from(vec![
            Span::styled(" pattern: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                display_safe(&search.pattern),
                Style::default().fg(Color::Cyan),
            ),
        ]));
    }

    let lines = preview_lines(app, preview_limit);
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn preview_lines(app: &App, preview_limit: usize) -> Vec<Line<'static>> {
    let Some(search) = &app.search else {
        return vec![
            Line::styled(
                "  Type a literal pattern above and press Enter.",
                Style::default().fg(Color::Gray),
            ),
            Line::raw(""),
            Line::styled(
                "  Matching lines are previewed here. /d deletes all of them; /s leaves them untouched.",
                Style::default().fg(Color::DarkGray),
            ),
        ];
    };

    if search.matches.is_empty() {
        return vec![
            Line::styled(
                format!("  No lines contain \"{}\".", display_safe(&search.pattern)),
                Style::default().fg(Color::Gray),
            ),
            Line::raw(""),
            Line::styled(
                "  Type another pattern, or /q to save and quit.",
                Style::default().fg(Color::DarkGray),
            ),
        ];
    }

    let line_number_width = app.document.lines.len().max(1).to_string().len();
    let mut lines = Vec::with_capacity(preview_limit + 1);

    for &index in search.matches.iter().take(preview_limit) {
        let text = &app.document.lines[index].content;
        let mut spans = vec![
            Span::styled(
                format!("  {:>line_number_width$} ", index + 1),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled("│ ", Style::default().fg(Color::DarkGray)),
        ];
        spans.extend(highlighted_spans(text, &search.pattern));
        lines.push(Line::from(spans));
    }

    let hidden = search.matches.len().saturating_sub(preview_limit);
    if hidden > 0 {
        lines.push(Line::styled(
            format!("  … {hidden} more matching line(s) not shown"),
            Style::default().fg(Color::DarkGray),
        ));
    }

    lines
}

fn highlighted_spans(text: &str, pattern: &str) -> Vec<Span<'static>> {
    let text = display_safe(text);
    let highlight = Style::default()
        .fg(Color::Black)
        .bg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let mut spans = Vec::new();
    let mut last = 0;

    for (start, found) in text.match_indices(pattern) {
        if start > last {
            spans.push(Span::raw(text[last..start].to_string()));
        }
        spans.push(Span::styled(found.to_string(), highlight));
        last = start + found.len();
    }

    if last < text.len() {
        spans.push(Span::raw(text[last..].to_string()));
    }

    spans
}

fn draw_input(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::bordered()
        .border_style(Style::default().fg(Color::Cyan))
        .title(Line::from(Span::styled(
            " Pattern or command ",
            Style::default().fg(Color::Cyan),
        )));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let prompt = "> ";
    let available = inner
        .width
        .saturating_sub(UnicodeWidthStr::width(prompt) as u16) as usize;
    if available == 0 || inner.height == 0 {
        return;
    }

    let value = app.input.value();
    let characters: Vec<char> = value.chars().collect();
    let cursor_characters = value[..app.input.cursor()].chars().count();
    let cursor_width: usize = characters[..cursor_characters]
        .iter()
        .map(|character| UnicodeWidthChar::width(*character).unwrap_or(0))
        .sum();

    let mut start = 0;
    if cursor_width >= available {
        let mut visible_width = 0;
        for index in (0..cursor_characters).rev() {
            let character_width = UnicodeWidthChar::width(characters[index]).unwrap_or(0);
            if visible_width + character_width + 1 > available {
                break;
            }
            visible_width += character_width;
            start = index;
        }
    }

    let visible: String = characters[start..].iter().collect();
    let cursor_prefix: String = characters[start..cursor_characters].iter().collect();
    let cursor_offset = UnicodeWidthStr::width(cursor_prefix.as_str()) as u16;

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                prompt,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(visible),
        ])),
        inner,
    );

    let cursor_x = inner.x + UnicodeWidthStr::width(prompt) as u16 + cursor_offset;
    if cursor_x < area.right() {
        frame.set_cursor_position((cursor_x, inner.y));
    }
}

fn draw_status(frame: &mut Frame, area: Rect, app: &App) {
    let (prefix, color) = match app.status.kind {
        StatusKind::Info => ("i  ", Color::Gray),
        StatusKind::Success => ("OK  ", Color::Green),
        StatusKind::Error => ("ERR  ", Color::Red),
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                prefix,
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                display_safe(&app.status.message),
                Style::default().fg(color),
            ),
        ])),
        area,
    );
}

fn draw_help(frame: &mut Frame, area: Rect) {
    let label = |text: &'static str, color: Color| {
        Span::styled(
            text,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )
    };
    let text = |value: &'static str| Span::styled(value, Style::default().fg(Color::DarkGray));

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            label("Enter", Color::Cyan),
            text(" search   "),
            label("/d", Color::Red),
            text(" delete   "),
            label("/s", Color::Green),
            text(" skip   "),
            label("/w <file>", Color::Cyan),
            text(" save   "),
            label("/q", Color::Yellow),
            text(" save & quit   "),
            label("Esc", Color::Gray),
            text(" quit"),
        ])),
        area,
    );
}

fn display_safe(text: &str) -> String {
    text.chars()
        .flat_map(|character| match character {
            '\t' => vec![' '; 4],
            character if character.is_control() => vec!['·'],
            character => vec![character],
        })
        .collect()
}
