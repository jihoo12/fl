use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crossterm::style::Color;

use crate::config::Config;

pub struct Entry {
    pub name: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub is_executable: bool,
    pub size: u64,
}

#[cfg(unix)]
pub fn is_exec_metadata(m: &fs::Metadata) -> bool {
    m.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
pub fn is_exec_metadata(_m: &fs::Metadata) -> bool {
    false
}

pub fn entry_color(entry: &Entry) -> Color {
    if entry.is_dir {
        Color::Blue
    } else if entry.is_symlink {
        Color::Cyan
    } else if entry.is_executable {
        Color::Green
    } else {
        Color::Reset
    }
}

pub fn load_entries(path: &Path, show_hidden: bool, config: &Config) -> Result<Vec<Entry>, String> {
    let rd = fs::read_dir(path).map_err(|e| format!("cannot read directory: {e}"))?;

    let mut entries: Vec<Entry> = rd
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if !show_hidden && name.starts_with('.') {
                return None;
            }

            let file_type = e.file_type().ok();
            let is_symlink = file_type.map(|t| t.is_symlink()).unwrap_or(false);

            let target_meta = fs::metadata(e.path()).ok();

            let is_dir = if is_symlink {
                target_meta.as_ref().map(|m| m.is_dir()).unwrap_or(false)
            } else {
                file_type.map(|t| t.is_dir()).unwrap_or(false)
            };

            if config.should_ignore(&name, is_dir) {
                return None;
            }

            let size = target_meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let is_executable = !is_dir
                && target_meta
                    .as_ref()
                    .map(is_exec_metadata)
                    .unwrap_or(false);

            Some(Entry {
                name,
                is_dir,
                is_symlink,
                is_executable,
                size,
            })
        })
        .collect();

    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(entries)
}

pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes}{}", UNITS[0])
    } else {
        format!("{size:.1}{}", UNITS[unit])
    }
}

pub fn truncate_to(s: &str, max: usize) -> String {
    let count = s.chars().count();
    if count <= max {
        s.to_string()
    } else if max <= 1 {
        s.chars().take(max).collect()
    } else {
        let keep = max - 1;
        let mut out: String = s.chars().take(keep).collect();
        out.push('…');
        out
    }
}
