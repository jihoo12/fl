use std::env;
use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute, queue,
    style::{self, Print},
    terminal::{self, ClearType},
};

struct Entry {
    name: String,
    is_dir: bool,
}

fn load_entries(path: &Path) -> Vec<Entry> {
    let mut entries: Vec<Entry> = fs::read_dir(path)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| {
                    let ft = e.file_type().ok();
                    Entry {
                        name: e.file_name().to_string_lossy().to_string(),
                        is_dir: ft.map(|t| t.is_dir()).unwrap_or(false),
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    entries
}

fn open_file(path: &Path) -> Result<(), Box<dyn Error>> {
    let editor = env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

    execute!(io::stdout(), terminal::LeaveAlternateScreen, cursor::Show)?;
    terminal::disable_raw_mode()?;

    Command::new(&editor).arg(path).status()?;

    terminal::enable_raw_mode()?;
    execute!(io::stdout(), terminal::EnterAlternateScreen, cursor::Hide)?;

    Ok(())
}

fn run_browser(initial_path: &Path) -> Result<(), Box<dyn Error>> {
    let mut path = initial_path.to_path_buf();
    let mut entries = load_entries(&path);
    let mut selected = 0usize;
    let mut scroll = 0usize;
    let mut stdout = io::stdout();

    loop {
        let (width, height) = terminal::size()?;
        let content_height = height.saturating_sub(4) as usize;

        queue!(stdout, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;

        let header = format!(" {}", path.display());
        writeln!(stdout, "{header}")?;
        for _ in 0..width {
            queue!(stdout, Print("-"))?;
        }
        queue!(stdout, cursor::MoveToNextLine(1))?;

        if selected >= scroll + content_height {
            scroll = selected.saturating_sub(content_height) + 1;
        }
        if selected < scroll {
            scroll = selected;
        }

        let visible = &entries[scroll..];
        for (i, entry) in visible.iter().take(content_height).enumerate() {
            let row = 2 + i as u16;
            queue!(stdout, cursor::MoveTo(0, row))?;

            let idx = scroll + i;
            if idx == selected {
                queue!(stdout, style::SetAttribute(style::Attribute::Reverse))?;
            }

            let display = if entry.is_dir {
                format!("{}/", entry.name)
            } else {
                entry.name.clone()
            };
            write!(stdout, "  {display}")?;

            if idx == selected {
                queue!(stdout, style::SetAttribute(style::Attribute::Reset))?;
            }
        }

        let footer_row = height.saturating_sub(1);
        queue!(stdout, cursor::MoveTo(0, footer_row))?;
        write!(stdout, "↑↓:nav  Enter:open  Backspace:parent  q:quit")?;

        stdout.flush()?;

        if let Event::Key(key) = event::read()? {
            let modified = key.modifiers.contains(KeyModifiers::SHIFT);
            match key.code {
                KeyCode::Up | KeyCode::Char('k') if !entries.is_empty() => {
                    selected = selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') if !entries.is_empty() => {
                    selected = (selected + 1).min(entries.len() - 1);
                }
                KeyCode::Home | KeyCode::Char('g') if !modified => {
                    selected = 0;
                }
                KeyCode::End => {
                    selected = entries.len().saturating_sub(1);
                }
                KeyCode::Char('G') if modified => {
                    selected = entries.len().saturating_sub(1);
                }
                KeyCode::PageUp => {
                    selected = selected.saturating_sub(content_height);
                }
                KeyCode::PageDown if !entries.is_empty() => {
                    selected = (selected + content_height).min(entries.len() - 1);
                }
                KeyCode::Enter => {
                    if entries.is_empty() {
                        continue;
                    }
                    let entry_name = entries[selected].name.clone();
                    let entry_is_dir = entries[selected].is_dir;
                    let new_path = path.join(&entry_name);
                    if entry_is_dir {
                        path = new_path;
                        entries = load_entries(&path);
                        selected = 0;
                        scroll = 0;
                    } else {
                        open_file(&new_path)?;
                        entries = load_entries(&path);
                        selected = selected.min(entries.len().saturating_sub(1));
                    }
                }
                KeyCode::Backspace => {
                    if let Some(parent) = path.parent() {
                        if parent.as_os_str().is_empty() {
                            path = PathBuf::from("/");
                        } else {
                            path = parent.to_path_buf();
                        }
                        entries = load_entries(&path);
                        selected = 0;
                        scroll = 0;
                    }
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    break;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let initial_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        env::current_dir()?
    };

    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

    let res = run_browser(&initial_path);

    execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show)?;
    terminal::disable_raw_mode()?;

    res
}
