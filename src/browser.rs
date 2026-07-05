use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{self},
};

use crate::config::Config;
use crate::entry::{self, Entry};
use crate::search;

pub enum Mode {
    Normal,
    Search(String),
    SearchRecursive(String),
    Rename(String),
    NewFile(String),
    NewDir(String),
    ConfirmDelete,
}

pub enum StatusKind {
    Success,
    Error,
    Info,
}

pub struct Browser {
    pub path: PathBuf,
    pub entries: Vec<Entry>,
    pub selected: usize,
    pub scroll: usize,
    pub show_hidden: bool,
    pub mode: Mode,
    pub status: Option<(StatusKind, String)>,
    pub positions: HashMap<PathBuf, (usize, usize)>,
    pub config: Config,
    pub show_search_results: bool,
    pub search_query: String,
}

impl Browser {
    pub fn new(initial_path: PathBuf, config: Config) -> Self {
        let show_hidden = false;
        let entries = entry::load_entries(&initial_path, show_hidden, &config).unwrap_or_default();
        Browser {
            path: initial_path,
            entries,
            selected: 0,
            scroll: 0,
            show_hidden,
            mode: Mode::Normal,
            status: None,
            positions: HashMap::new(),
            config,
            show_search_results: false,
            search_query: String::new(),
        }
    }

    pub fn set_status(&mut self, kind: StatusKind, msg: impl Into<String>) {
        self.status = Some((kind, msg.into()));
    }

    pub fn reload(&mut self) {
        match entry::load_entries(&self.path, self.show_hidden, &self.config) {
            Ok(entries) => {
                self.entries = entries;
                if self.selected >= self.entries.len() {
                    self.selected = self.entries.len().saturating_sub(1);
                }
            }
            Err(e) => {
                self.entries.clear();
                self.set_status(StatusKind::Error, e);
            }
        }
    }

    pub fn enter_dir(&mut self, new_path: PathBuf) {
        self.positions
            .insert(self.path.clone(), (self.selected, self.scroll));
        self.path = new_path;
        match entry::load_entries(&self.path, self.show_hidden, &self.config) {
            Ok(entries) => self.entries = entries,
            Err(e) => {
                self.entries.clear();
                self.set_status(StatusKind::Error, e);
            }
        }
        if let Some(&(sel, scr)) = self.positions.get(&self.path) {
            self.selected = sel.min(self.entries.len().saturating_sub(1));
            self.scroll = scr;
        } else {
            self.selected = 0;
            self.scroll = 0;
        }
    }

    pub fn go_to_parent(&mut self) {
        if let Some(parent) = self.path.parent() {
            let parent = if parent.as_os_str().is_empty() {
                PathBuf::from("/")
            } else {
                parent.to_path_buf()
            };
            self.enter_dir(parent);
        }
    }

    pub fn selected_entry_name(&self) -> Option<String> {
        self.entries.get(self.selected).map(|e| e.name.clone())
    }
}

fn search_jump(entries: &[Entry], selected: &mut usize, buf: &str) {
    if buf.is_empty() {
        return;
    }
    let needle = buf.to_lowercase();
    if let Some(idx) = entries
        .iter()
        .position(|e| e.name.to_lowercase().starts_with(&needle))
    {
        *selected = idx;
    }
}

fn open_file(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

    execute!(io::stdout(), terminal::LeaveAlternateScreen, cursor::Show)?;
    terminal::disable_raw_mode()?;

    let status = Command::new(&editor).arg(path).status();

    terminal::enable_raw_mode()?;
    execute!(io::stdout(), terminal::EnterAlternateScreen, cursor::Hide)?;

    status?;
    Ok(())
}

pub fn handle_normal_key(app: &mut Browser, key: event::KeyEvent) -> Result<bool, Box<dyn std::error::Error>> {
    let modified = key.modifiers.contains(KeyModifiers::SHIFT);
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Up | KeyCode::Char('k') if !app.entries.is_empty() => {
            app.selected = app.selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') if !app.entries.is_empty() => {
            app.selected = (app.selected + 1).min(app.entries.len() - 1);
        }
        KeyCode::Home | KeyCode::Char('g') if !modified => {
            app.selected = 0;
        }
        KeyCode::End => {
            app.selected = app.entries.len().saturating_sub(1);
        }
        KeyCode::Char('G') if modified => {
            app.selected = app.entries.len().saturating_sub(1);
        }
        KeyCode::PageUp => {
            let (_, height) = terminal::size()?;
            let content_height = height.saturating_sub(4) as usize;
            app.selected = app.selected.saturating_sub(content_height);
        }
        KeyCode::PageDown if !app.entries.is_empty() => {
            let (_, height) = terminal::size()?;
            let content_height = height.saturating_sub(4) as usize;
            app.selected = (app.selected + content_height).min(app.entries.len() - 1);
        }
        KeyCode::Enter => {
            if app.entries.is_empty() {
                return Ok(false);
            }
            let entry_name = app.entries[app.selected].name.clone();
            let entry_is_dir = app.entries[app.selected].is_dir;
            let new_path = app.path.join(&entry_name);
            if entry_is_dir {
                if app.show_search_results {
                    app.show_search_results = false;
                }
                app.enter_dir(new_path);
            } else {
                match open_file(&new_path) {
                    Ok(()) => {}
                    Err(e) => app.set_status(StatusKind::Error, format!("could not open editor: {e}")),
                }
                if app.show_search_results {
                    app.show_search_results = false;
                }
                app.reload();
            }
        }
        KeyCode::Backspace => {
            if app.show_search_results {
                app.show_search_results = false;
                app.reload();
            } else {
                app.go_to_parent();
            }
        }
        KeyCode::Char('/') => {
            app.mode = Mode::Search(String::new());
        }
        KeyCode::Char('f') if ctrl => {
            app.mode = Mode::SearchRecursive(String::new());
        }
        KeyCode::Char('r') if !app.entries.is_empty() => {
            let current = app.entries[app.selected].name.clone();
            app.mode = Mode::Rename(current);
        }
        KeyCode::Char('a') => {
            app.mode = Mode::NewFile(String::new());
        }
        KeyCode::Char('m') => {
            app.mode = Mode::NewDir(String::new());
        }
        KeyCode::Char('x') | KeyCode::Delete if !app.entries.is_empty() => {
            app.mode = Mode::ConfirmDelete;
        }
        KeyCode::Char('.') => {
            app.show_hidden = !app.show_hidden;
            app.reload();
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            return Ok(true);
        }
        _ => {}
    }
    Ok(false)
}

pub fn handle_key(app: &mut Browser, key: event::KeyEvent) -> Result<bool, Box<dyn std::error::Error>> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Ok(true);
    }

    let mode = std::mem::replace(&mut app.mode, Mode::Normal);
    match mode {
        Mode::Normal => {
            app.mode = Mode::Normal;
            return handle_normal_key(app, key);
        }
        Mode::Search(mut buf) => match key.code {
            KeyCode::Char(c) => {
                buf.push(c);
                search_jump(&app.entries, &mut app.selected, &buf);
                app.mode = Mode::Search(buf);
            }
            KeyCode::Backspace => {
                buf.pop();
                search_jump(&app.entries, &mut app.selected, &buf);
                app.mode = Mode::Search(buf);
            }
            KeyCode::Enter | KeyCode::Esc => {
                app.mode = Mode::Normal;
            }
            _ => {
                app.mode = Mode::Search(buf);
            }
        },
        Mode::SearchRecursive(mut buf) => match key.code {
            KeyCode::Char(c) => {
                buf.push(c);
                app.mode = Mode::SearchRecursive(buf);
            }
            KeyCode::Backspace => {
                buf.pop();
                app.mode = Mode::SearchRecursive(buf);
            }
            KeyCode::Enter => {
                if !buf.is_empty() {
                    let results = search::recursive_search(
                        &app.path,
                        &buf,
                        app.show_hidden,
                        &app.config,
                    );
                    let count = results.len();
                    app.entries = results;
                    app.selected = 0;
                    app.scroll = 0;
                    app.show_search_results = true;
                    app.search_query = buf;
                    app.set_status(StatusKind::Success, format!("found {count} result(s)"));
                }
                app.mode = Mode::Normal;
            }
            KeyCode::Esc => {
                app.mode = Mode::Normal;
            }
            _ => {
                app.mode = Mode::SearchRecursive(buf);
            }
        },
        Mode::Rename(mut buf) => match key.code {
            KeyCode::Char(c) => {
                buf.push(c);
                app.mode = Mode::Rename(buf);
            }
            KeyCode::Backspace => {
                buf.pop();
                app.mode = Mode::Rename(buf);
            }
            KeyCode::Enter => {
                if let Some(old_name) = app.selected_entry_name() {
                    let old_path = app.path.join(&old_name);
                    let new_path = app.path.join(&buf);
                    match fs::rename(&old_path, &new_path) {
                        Ok(()) => app.set_status(StatusKind::Success, format!("renamed to '{buf}'")),
                        Err(e) => app.set_status(StatusKind::Error, format!("rename failed: {e}")),
                    }
                    app.reload();
                }
                app.mode = Mode::Normal;
            }
            KeyCode::Esc => {
                app.mode = Mode::Normal;
            }
            _ => {
                app.mode = Mode::Rename(buf);
            }
        },
        Mode::NewFile(mut buf) => match key.code {
            KeyCode::Char(c) => {
                buf.push(c);
                app.mode = Mode::NewFile(buf);
            }
            KeyCode::Backspace => {
                buf.pop();
                app.mode = Mode::NewFile(buf);
            }
            KeyCode::Enter => {
                if !buf.is_empty() {
                    let new_path = app.path.join(&buf);
                    match fs::File::create(&new_path) {
                        Ok(_) => app.set_status(StatusKind::Success, format!("created '{buf}'")),
                        Err(e) => app.set_status(StatusKind::Error, format!("create failed: {e}")),
                    }
                    app.reload();
                }
                app.mode = Mode::Normal;
            }
            KeyCode::Esc => {
                app.mode = Mode::Normal;
            }
            _ => {
                app.mode = Mode::NewFile(buf);
            }
        },
        Mode::NewDir(mut buf) => match key.code {
            KeyCode::Char(c) => {
                buf.push(c);
                app.mode = Mode::NewDir(buf);
            }
            KeyCode::Backspace => {
                buf.pop();
                app.mode = Mode::NewDir(buf);
            }
            KeyCode::Enter => {
                if !buf.is_empty() {
                    let new_path = app.path.join(&buf);
                    match fs::create_dir(&new_path) {
                        Ok(()) => app.set_status(StatusKind::Success, format!("created directory '{buf}'")),
                        Err(e) => app.set_status(StatusKind::Error, format!("mkdir failed: {e}")),
                    }
                    app.reload();
                }
                app.mode = Mode::Normal;
            }
            KeyCode::Esc => {
                app.mode = Mode::Normal;
            }
            _ => {
                app.mode = Mode::NewDir(buf);
            }
        },
        Mode::ConfirmDelete => {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    if let Some(name) = app.selected_entry_name() {
                        let target = app.path.join(&name);
                        let is_dir = app
                            .entries
                            .get(app.selected)
                            .map(|e| e.is_dir)
                            .unwrap_or(false);
                        let result = if is_dir {
                            fs::remove_dir_all(&target)
                        } else {
                            fs::remove_file(&target)
                        };
                        match result {
                            Ok(()) => app.set_status(StatusKind::Success, format!("deleted '{name}'")),
                            Err(e) => app.set_status(StatusKind::Error, format!("delete failed: {e}")),
                        }
                        app.reload();
                    }
                }
                _ => {
                    app.set_status(StatusKind::Info, "delete cancelled");
                }
            }
            app.mode = Mode::Normal;
        }
    }
    Ok(false)
}

pub fn run_browser(app: &mut Browser) -> Result<(), Box<dyn std::error::Error>> {
    use crate::ui::draw;
    let mut stdout = io::stdout();

    loop {
        draw(app, &mut stdout)?;

        let ev = event::read()?;
        if matches!(app.mode, Mode::Normal) {
            app.status = None;
        }

        if let Event::Key(key) = ev {
            if handle_key(app, key)? {
                break;
            }
        }
    }

    Ok(())
}
