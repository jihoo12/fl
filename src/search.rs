use std::fs;
use std::path::Path;

use crate::config::Config;
use crate::entry::{is_exec_metadata, Entry};

/// Recursively walk all subdirectories of `root` and return every entry whose
/// name (the last path component) contains `query` (case-insensitive).
/// Each result stores its relative path (from `root`) in `Entry.name`.
pub fn recursive_search(
    root: &Path,
    query: &str,
    show_hidden: bool,
    config: &Config,
) -> Vec<Entry> {
    let needle = query.to_lowercase();
    let mut results = Vec::new();
    walk(root, root, &needle, show_hidden, config, &mut results);
    results.sort_by(|a, b| a.name.cmp(&b.name));
    results
}

fn walk(
    root: &Path,
    dir: &Path,
    needle: &str,
    show_hidden: bool,
    config: &Config,
    results: &mut Vec<Entry>,
) {
    let rd = match fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return,
    };

    for entry in rd.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();

        if !show_hidden && name.starts_with('.') {
            continue;
        }

        let file_type = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };

        let is_symlink = file_type.is_symlink();
        let target_meta = fs::metadata(entry.path()).ok();
        let is_dir = if is_symlink {
            target_meta.as_ref().map(|m| m.is_dir()).unwrap_or(false)
        } else {
            file_type.is_dir()
        };

        if config.should_ignore(&name, is_dir) {
            continue;
        }

        if name.to_lowercase().contains(needle) {
            let path = entry.path();
            let rel = path.strip_prefix(root).unwrap_or(&path);
            let rel_str = rel.to_string_lossy().to_string();

            let size = target_meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let is_executable = !is_dir
                && target_meta
                    .as_ref()
                    .map(is_exec_metadata)
                    .unwrap_or(false);

            results.push(Entry {
                name: rel_str,
                is_dir,
                is_symlink,
                is_executable,
                size,
            });
        }

        if is_dir {
            walk(root, &entry.path(), needle, show_hidden, config, results);
        }
    }
}
