use chrono::{DateTime, Local};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextLine {
    pub content: String,
    pub ending: String,
}

#[derive(Debug, Clone)]
pub struct Document {
    pub path: PathBuf,
    pub lines: Vec<TextLine>,
    pub deleted: usize,
}

impl Document {
    pub fn load(path: PathBuf) -> io::Result<Self> {
        let text = fs::read_to_string(&path)?;
        Ok(Self {
            path,
            lines: parse_lines(&text),
            deleted: 0,
        })
    }

    pub fn matching_indices(&self, pattern: &str) -> Vec<usize> {
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(index, line)| line.content.contains(pattern).then_some(index))
            .collect()
    }

    pub fn delete_matching(&mut self, pattern: &str) -> usize {
        let before = self.lines.len();
        self.lines.retain(|line| !line.content.contains(pattern));
        let removed = before - self.lines.len();
        self.deleted += removed;
        removed
    }

    pub fn to_text(&self) -> String {
        let capacity = self
            .lines
            .iter()
            .map(|line| line.content.len() + line.ending.len())
            .sum();
        let mut text = String::with_capacity(capacity);
        for line in &self.lines {
            text.push_str(&line.content);
            text.push_str(&line.ending);
        }
        text
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.to_text())
    }

    pub fn save_timestamped(&self, now: DateTime<Local>) -> io::Result<PathBuf> {
        let stamp = now.format("%Y-%m-%d_%H-%M-%S").to_string();
        let path = self.timestamped_path_for(&stamp);
        self.save(&path)?;
        Ok(path)
    }

    fn timestamped_path_for(&self, stamp: &str) -> PathBuf {
        let parent = self.path.parent().unwrap_or_else(|| Path::new(""));
        let stem = self
            .path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .filter(|stem| !stem.is_empty())
            .unwrap_or_else(|| "output".to_string());
        let extension = self
            .path
            .extension()
            .map(|extension| extension.to_string_lossy().into_owned())
            .filter(|extension| !extension.is_empty());

        let mut suffix = String::new();
        let mut attempt = 2;
        loop {
            let name = match &extension {
                Some(extension) => format!("{stem}_{stamp}{suffix}.{extension}"),
                None => format!("{stem}_{stamp}{suffix}"),
            };
            let candidate = parent.join(name);
            if !candidate.exists() {
                return candidate;
            }
            suffix = format!("_{attempt}");
            attempt += 1;
        }
    }
}

fn parse_lines(text: &str) -> Vec<TextLine> {
    text.split_inclusive('\n')
        .map(|chunk| {
            let (content, ending) = if let Some(content) = chunk.strip_suffix("\r\n") {
                (content, "\r\n")
            } else if let Some(content) = chunk.strip_suffix('\n') {
                (content, "\n")
            } else {
                (chunk, "")
            };
            TextLine {
                content: content.to_string(),
                ending: ending.to_string(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(text: &str) -> Document {
        Document {
            path: PathBuf::from("input.txt"),
            lines: parse_lines(text),
            deleted: 0,
        }
    }

    #[test]
    fn parses_lf_lines_without_inventing_a_final_line() {
        let doc = document("one\ntwo\n");
        assert_eq!(doc.lines.len(), 2);
        assert_eq!(doc.lines[0].content, "one");
        assert_eq!(doc.lines[0].ending, "\n");
        assert_eq!(doc.lines[1].content, "two");
        assert_eq!(doc.lines[1].ending, "\n");
    }

    #[test]
    fn preserves_crlf_and_missing_final_newline() {
        let doc = document("one\r\ntwo\r\nthree");
        assert_eq!(doc.lines[1].ending, "\r\n");
        assert_eq!(doc.lines[2].ending, "");
        assert_eq!(doc.to_text(), "one\r\ntwo\r\nthree");
    }

    #[test]
    fn empty_input_has_no_lines() {
        let doc = document("");
        assert!(doc.lines.is_empty());
        assert_eq!(doc.to_text(), "");
    }

    #[test]
    fn matches_are_literal_and_case_sensitive() {
        let doc = document("Needle\nneedle\nNeedle in a haystack\n");
        assert_eq!(doc.matching_indices("Needle"), vec![0, 2]);
        assert_eq!(doc.matching_indices("needle"), vec![1]);
        assert_eq!(doc.matching_indices("in a"), vec![2]);
        assert!(doc.matching_indices("regex.*").is_empty());
    }

    #[test]
    fn deletion_removes_every_matching_line_and_preserves_the_rest() {
        let mut doc = document("keep\r\nremove me\r\nkeep too\r\nremove me again");
        let removed = doc.delete_matching("remove me");
        assert_eq!(removed, 2);
        assert_eq!(doc.deleted, 2);
        assert_eq!(doc.to_text(), "keep\r\nkeep too\r\n");
    }

    #[test]
    fn timestamped_name_lives_next_to_the_input_and_avoids_collisions() {
        let dir = std::env::temp_dir().join(format!(
            "joli-document-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&dir).unwrap();
        let input = dir.join("notes.txt");
        fs::write(&input, "hello\n").unwrap();

        let mut doc = document("hello\n");
        doc.path = input;
        let first = doc.timestamped_path_for("2026-11-05_12-34-56");
        assert_eq!(
            first.file_name().unwrap().to_string_lossy(),
            "notes_2026-11-05_12-34-56.txt"
        );
        assert!(!first.exists());

        fs::write(&first, "").unwrap();
        let second = doc.timestamped_path_for("2026-11-05_12-34-56");
        assert_eq!(
            second.file_name().unwrap().to_string_lossy(),
            "notes_2026-11-05_12-34-56_2.txt"
        );

        fs::remove_dir_all(dir).unwrap();
    }
}
