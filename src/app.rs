use std::path::PathBuf;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::document::Document;

pub const PREVIEW_LIMIT: usize = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitAction {
    Quit,
    SaveAndQuit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Info,
    Success,
    Error,
}

#[derive(Debug, Clone)]
pub struct Status {
    pub message: String,
    pub kind: StatusKind,
}

impl Status {
    fn info(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: StatusKind::Info,
        }
    }

    fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: StatusKind::Success,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: StatusKind::Error,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Search {
    pub pattern: String,
    pub matches: Vec<usize>,
}

#[derive(Debug, Default)]
pub struct TextInput {
    value: String,
    cursor: usize,
}

impl TextInput {
    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn insert_char(&mut self, character: char) {
        self.value.insert(self.cursor, character);
        self.cursor += character.len_utf8();
    }

    pub fn insert_str(&mut self, text: &str) {
        self.value.insert_str(self.cursor, text);
        self.cursor += text.len();
    }

    pub fn backspace(&mut self) {
        if let Some(previous) = self.previous_boundary() {
            self.value.remove(previous);
            self.cursor = previous;
        }
    }

    pub fn delete(&mut self) {
        if self.cursor < self.value.len() {
            self.value.remove(self.cursor);
        }
    }

    pub fn move_left(&mut self) {
        if let Some(previous) = self.previous_boundary() {
            self.cursor = previous;
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.value.len() {
            self.cursor = self.next_boundary();
        }
    }

    pub fn home(&mut self) {
        self.cursor = 0;
    }

    pub fn end(&mut self) {
        self.cursor = self.value.len();
    }

    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }

    pub fn take_trimmed(&mut self) -> String {
        let value = self.value.trim().to_string();
        self.clear();
        value
    }

    fn previous_boundary(&self) -> Option<usize> {
        if self.cursor == 0 {
            return None;
        }
        let mut index = self.cursor - 1;
        while !self.value.is_char_boundary(index) {
            index -= 1;
        }
        Some(index)
    }

    fn next_boundary(&self) -> usize {
        let mut index = self.cursor + 1;
        while index < self.value.len() && !self.value.is_char_boundary(index) {
            index += 1;
        }
        index
    }
}

pub struct App {
    pub document: Document,
    pub input: TextInput,
    pub search: Option<Search>,
    pub status: Status,
    pub dirty: bool,
    pub saved_to: Option<PathBuf>,
}

impl App {
    pub fn new(document: Document) -> Self {
        Self {
            document,
            input: TextInput::default(),
            search: None,
            status: Status::info("Type a literal pattern and press Enter to find matching lines."),
            dirty: false,
            saved_to: None,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<ExitAction> {
        if key.kind == KeyEventKind::Release {
            return None;
        }

        match key.code {
            KeyCode::Esc => return Some(ExitAction::Quit),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Some(ExitAction::Quit);
            }
            KeyCode::Enter => return self.submit(),
            KeyCode::Backspace => self.input.backspace(),
            KeyCode::Delete => self.input.delete(),
            KeyCode::Left => self.input.move_left(),
            KeyCode::Right => self.input.move_right(),
            KeyCode::Home => self.input.home(),
            KeyCode::End => self.input.end(),
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.home();
            }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.end();
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.clear();
            }
            KeyCode::Char(character)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                    && !character.is_control() =>
            {
                self.input.insert_char(character);
            }
            _ => {}
        }

        None
    }

    pub fn paste(&mut self, text: &str) {
        let clean: String = text
            .chars()
            .filter_map(|character| {
                if matches!(character, '\n' | '\r' | '\t') {
                    Some(' ')
                } else if character.is_control() {
                    None
                } else {
                    Some(character)
                }
            })
            .collect();
        self.input.insert_str(&clean);
    }

    fn submit(&mut self) -> Option<ExitAction> {
        let input = self.input.take_trimmed();

        if input.is_empty() {
            self.search = None;
            self.status = Status::info("Enter a plain-text pattern to search for matching lines.");
            return None;
        }

        match input.as_str() {
            "/q" => return Some(ExitAction::SaveAndQuit),
            "/d" => {
                self.delete_matches();
                return None;
            }
            "/s" => {
                self.skip_matches();
                return None;
            }
            "/w" => {
                self.status = Status::error("Usage: /w <filename>");
                return None;
            }
            _ => {}
        }

        if let Some(argument) = argument_after(&input, "/w") {
            self.save_explicit(argument);
            return None;
        }

        self.run_search(input);
        None
    }

    fn run_search(&mut self, pattern: String) {
        let matches = self.document.matching_indices(&pattern);
        let count = matches.len();
        self.status = if count == 0 {
            Status::info(format!("No lines contain \"{pattern}\"."))
        } else {
            Status::info(format!(
                "Found {count} matching line(s). /d deletes them; /s skips them."
            ))
        };
        self.search = Some(Search { pattern, matches });
    }

    fn delete_matches(&mut self) {
        let Some(search) = self.search.take() else {
            self.status = Status::error("Search for a pattern before using /d.");
            return;
        };

        if search.matches.is_empty() {
            self.status = Status::info("The current search has no matching lines to delete.");
            return;
        }

        let removed = self.document.delete_matching(&search.pattern);
        self.dirty = true;
        self.status = Status::success(format!(
            "Deleted {removed} matching line(s). {} lines remain.",
            self.document.lines.len()
        ));
    }

    fn skip_matches(&mut self) {
        let Some(search) = self.search.take() else {
            self.status = Status::info("No active match set to skip.");
            return;
        };

        if search.matches.is_empty() {
            self.status = Status::info("The current search has no matches; nothing to skip.");
        } else {
            self.status = Status::success(format!(
                "Skipped {} matching line(s).",
                search.matches.len()
            ));
        }
    }

    fn save_explicit(&mut self, argument: &str) {
        let target = unquote(argument);
        if target.is_empty() {
            self.status = Status::error("Usage: /w <filename>");
            return;
        }

        let path = PathBuf::from(target);
        match self.document.save(&path) {
            Ok(()) => {
                self.dirty = false;
                self.saved_to = Some(path.clone());
                self.status = Status::success(format!("Saved to {}", path.display()));
            }
            Err(error) => {
                self.status = Status::error(format!("Could not save {}: {error}", path.display()));
            }
        }
    }
}

fn argument_after<'a>(input: &'a str, command: &str) -> Option<&'a str> {
    let rest = input.strip_prefix(command)?;
    if rest.is_empty() || !rest.chars().next().is_some_and(char::is_whitespace) {
        return None;
    }
    Some(rest.trim())
}

fn unquote(value: &str) -> &str {
    if value.len() >= 2 {
        let first = value.as_bytes()[0];
        let last = value.as_bytes()[value.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return &value[1..value.len() - 1];
        }
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::TextLine;

    fn app_with(lines: &[&str]) -> App {
        let document = Document {
            path: PathBuf::from("input.txt"),
            lines: lines
                .iter()
                .map(|line| TextLine {
                    content: (*line).to_string(),
                    ending: "\n".to_string(),
                })
                .collect(),
            deleted: 0,
        };
        App::new(document)
    }

    fn enter(app: &mut App, input: &str) -> Option<ExitAction> {
        app.paste(input);
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
    }

    #[test]
    fn search_populates_matches_and_clears_the_input() {
        let mut app = app_with(&["needle", "haystack", "another needle"]);
        enter(&mut app, "needle");
        let search = app.search.as_ref().unwrap();
        assert_eq!(search.pattern, "needle");
        assert_eq!(search.matches, vec![0, 2]);
        assert_eq!(app.input.value(), "");
    }

    #[test]
    fn delete_removes_all_matching_lines_and_marks_the_document_dirty() {
        let mut app = app_with(&["needle", "haystack", "another needle"]);
        enter(&mut app, "needle");
        let action = enter(&mut app, "/d");
        assert!(action.is_none());
        assert_eq!(app.document.lines.len(), 1);
        assert_eq!(app.document.lines[0].content, "haystack");
        assert_eq!(app.document.deleted, 2);
        assert!(app.dirty);
        assert!(app.search.is_none());
    }

    #[test]
    fn skip_leaves_the_document_untouched() {
        let mut app = app_with(&["needle", "haystack", "another needle"]);
        enter(&mut app, "needle");
        enter(&mut app, "/s");
        assert_eq!(app.document.lines.len(), 3);
        assert!(!app.dirty);
        assert!(app.search.is_none());
    }

    #[test]
    fn quit_command_is_only_matched_without_extra_text() {
        let mut app = app_with(&["x"]);
        assert_eq!(enter(&mut app, "/q"), Some(ExitAction::SaveAndQuit));

        let mut app = app_with(&["/q suffix"]);
        assert_eq!(enter(&mut app, "/q suffix"), None);
        assert_eq!(app.search.as_ref().unwrap().pattern, "/q suffix");
    }

    #[test]
    fn write_command_requires_whitespace_before_the_filename() {
        assert_eq!(argument_after("/w notes.txt", "/w"), Some("notes.txt"));
        assert_eq!(argument_after("/w  notes.txt  ", "/w"), Some("notes.txt"));
        assert_eq!(argument_after("/w", "/w"), None);
        assert_eq!(argument_after("/wrong", "/w"), None);
    }

    #[test]
    fn text_input_edits_at_character_boundaries() {
        let mut input = TextInput::default();
        input.insert_str("héllo");
        input.end();
        input.backspace();
        assert_eq!(input.value(), "héll");
        input.home();
        input.delete();
        assert_eq!(input.value(), "éll");
        input.move_right();
        input.insert_char('x');
        assert_eq!(input.value(), "éxll");
    }
}
