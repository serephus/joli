mod app;
mod document;
mod ui;

use std::env;
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use chrono::Local;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event};

use crate::app::{App, ExitAction};
use crate::document::Document;

const USAGE: &str = "\
joli - interactive literal line filter

Usage:
  joli <FILE>

Commands inside the TUI:
  <text>        Search for lines containing the exact, case-sensitive text
  /d            Delete every line in the current match set
  /s            Skip the current match set without changing the file
  /w <FILE>     Save the working text to FILE and keep running
  /q            Save to a new timestamped file next to the input and quit
  Esc / Ctrl+C  Quit; unsaved changes are auto-saved

Notes:
  - Matching is plain, literal, case-sensitive text (not a regex).
  - Up to 5 matching lines are previewed.
  - /q always creates a new file named NAME_YYYY-MM-DD_HH-MM-SS.ext.
  - On Esc/Ctrl+C, unsaved changes are saved to a new timestamped file next
    to the input. The input file itself is never modified.
";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() == 1 && matches!(args[0].as_str(), "-h" | "--help") {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }

    if args.len() != 1 {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }

    run(PathBuf::from(&args[0]))
}

fn run(path: PathBuf) -> ExitCode {
    let document = match Document::load(path.clone()) {
        Ok(document) => document,
        Err(error) => {
            eprintln!("joli: could not read {}: {error}", path.display());
            return ExitCode::FAILURE;
        }
    };

    let mut app = App::new(document);
    let mut terminal = match ratatui::try_init() {
        Ok(terminal) => terminal,
        Err(error) => {
            eprintln!("joli: could not initialize the terminal: {error}");
            return ExitCode::FAILURE;
        }
    };

    let loop_result = event_loop(&mut terminal, &mut app);
    ratatui::restore();

    let mut successful = true;
    let action = match loop_result {
        Ok(action) => Some(action),
        Err(error) => {
            eprintln!("joli: terminal error: {error}");
            successful = false;
            None
        }
    };

    if !save_on_exit(&app, action) {
        successful = false;
    }

    if successful {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<ExitAction> {
    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;

        match event::read()? {
            Event::Key(key) => {
                if let Some(action) = app.handle_key(key) {
                    return Ok(action);
                }
            }
            Event::Paste(text) => app.paste(&text),
            _ => {}
        }
    }
}

fn save_on_exit(app: &App, action: Option<ExitAction>) -> bool {
    let needs_timestamped_save = matches!(action, Some(ExitAction::SaveAndQuit)) || app.dirty;

    if !needs_timestamped_save {
        return true;
    }

    match app.document.save_timestamped(Local::now()) {
        Ok(path) => {
            if matches!(action, Some(ExitAction::SaveAndQuit)) {
                println!("Saved to {}", path.display());
            } else {
                println!("Unsaved changes saved to {}", path.display());
            }
            true
        }
        Err(error) => {
            eprintln!("joli: could not save the processed text: {error}");
            false
        }
    }
}
