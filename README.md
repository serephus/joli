# joli

A small TUI for finding and deleting lines that contain an exact, case-sensitive
plain-text pattern. It previews up to five matching lines and never modifies the
input file.

## Build and run

```sh
cargo build --release
cargo run --release -- path/to/input.txt
```

The input file must be UTF-8 plain text.

## Usage

Type a word or phrase and press `Enter`. The header shows how many lines contain
that literal text, and the preview shows up to five of them with the matches
highlighted.

| Input | Action |
| --- | --- |
| `<text>` | Search for the exact, case-sensitive text |
| `/d` | Delete every line in the current match set |
| `/s` | Skip the current match set, leaving the text unchanged |
| `/w <FILE>` | Save the working text to `<FILE>` and keep running |
| `/q` | Save to a timestamped file next to the input, then quit |
| `Esc` / `Ctrl+C` | Quit; unsaved changes are auto-saved |

Typing a new pattern after a search also skips the previous match set. Search text
is literal: characters such as `.` or `*` have no special meaning.

## Saving

- `/w <FILE>` writes exactly to the path you choose. Existing files are overwritten.
- `/q` always creates a new file next to the input, named
  `NAME_YYYY-MM-DD_HH-MM-SS.ext` (for example `notes_2026-11-05_14-30-00.txt`).
- On `Esc`/`Ctrl+C`, only unsaved changes are written, and only to a new
  timestamped file next to the input. Use `/w` again if you want to update a
  specific file after further deletions.
- Line endings, including mixed CRLF/LF and a missing final newline, are preserved
  for the lines that remain.
- The original input file is never written unless you explicitly pass it to `/w`.

## Development

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```
