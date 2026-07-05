use std::io::{self, Write};

use crossterm::{
    cursor, queue,
    style::{Attribute, Color, Print, SetAttribute, SetForegroundColor},
    terminal,
};

use crate::browser::{Browser, Mode, StatusKind};
use crate::entry::{entry_color, human_size, truncate_to};

pub fn draw(app: &mut Browser, stdout: &mut io::Stdout) -> Result<(), Box<dyn std::error::Error>> {
    let (width, height) = terminal::size()?;
    let width = width as usize;
    let content_height = height.saturating_sub(4) as usize;

    queue!(stdout, terminal::Clear(crossterm::terminal::ClearType::All), cursor::MoveTo(0, 0))?;

    // --- Title bar ---
    let position_info = if app.entries.is_empty() {
        "empty".to_string()
    } else {
        format!("{}/{}", app.selected + 1, app.entries.len())
    };
    let header_raw = if app.show_search_results {
        format!(
            " search: {}  ({})",
            app.search_query, position_info
        )
    } else {
        let hidden_flag = if app.show_hidden { "  [hidden: on]" } else { "" };
        format!(" {}  ({}){}", app.path.display(), position_info, hidden_flag)
    };
    let header_trunc = truncate_to(&header_raw, width);
    let header_padded = format!("{:<width$}", header_trunc, width = width);
    queue!(stdout, SetAttribute(Attribute::Reverse), SetAttribute(Attribute::Bold))?;
    write!(stdout, "{header_padded}")?;
    queue!(
        stdout,
        SetAttribute(Attribute::Reset),
        cursor::MoveToNextLine(1)
    )?;

    // --- Separator ---
    queue!(stdout, SetForegroundColor(Color::DarkGrey))?;
    for _ in 0..width {
        queue!(stdout, Print("─"))?;
    }
    queue!(
        stdout,
        SetForegroundColor(Color::Reset),
        cursor::MoveToNextLine(1)
    )?;

    // --- List ---
    if app.selected >= app.scroll + content_height {
        app.scroll = app.selected.saturating_sub(content_height) + 1;
    }
    if app.selected < app.scroll {
        app.scroll = app.selected;
    }

    let scrollbar = app.entries.len() > content_height && content_height > 0;
    let size_col_width = 8usize;
    let reserved = 4 + size_col_width + if scrollbar { 1 } else { 0 };
    let name_budget = width.saturating_sub(reserved).max(1);

    let (thumb_pos, thumb_len) = if scrollbar {
        let max_scroll = app.entries.len() - content_height;
        let thumb_len = ((content_height * content_height) / app.entries.len()).max(1);
        let thumb_len = thumb_len.min(content_height);
        let denom = max_scroll.max(1);
        let range = content_height.saturating_sub(thumb_len).max(1);
        let pos = (app.scroll * range) / denom;
        (pos, thumb_len)
    } else {
        (0, 0)
    };

    let visible = if app.scroll < app.entries.len() {
        &app.entries[app.scroll..]
    } else {
        &app.entries[0..0]
    };
    for i in 0..content_height {
        let row = 2 + i as u16;
        let idx = app.scroll + i;

        queue!(stdout, cursor::MoveTo(0, row))?;

        if let Some(entry) = visible.get(i) {
            let selected = idx == app.selected;
            if selected {
                queue!(stdout, SetAttribute(Attribute::Reverse))?;
            }

            let suffix = if entry.is_dir {
                "/"
            } else if entry.is_symlink {
                "@"
            } else if entry.is_executable {
                "*"
            } else {
                ""
            };
            let name_display = truncate_to(&format!("{}{}", entry.name, suffix), name_budget);
            let size_display = if entry.is_dir {
                String::new()
            } else {
                human_size(entry.size)
            };

            let fg = if selected { Color::Reset } else { entry_color(entry) };
            queue!(stdout, SetForegroundColor(fg))?;
            write!(stdout, "  {:<name_w$}", name_display, name_w = name_budget)?;
            queue!(stdout, SetForegroundColor(Color::Reset))?;
            write!(stdout, "{:>size_w$}", size_display, size_w = size_col_width)?;

            if selected {
                queue!(stdout, SetAttribute(Attribute::Reset))?;
            }
        } else {
            write!(stdout, "{:width$}", "", width = width.saturating_sub(if scrollbar { 1 } else { 0 }))?;
        }

        if scrollbar {
            let is_thumb = i >= thumb_pos && i < thumb_pos + thumb_len;
            let ch = if is_thumb { "█" } else { "│" };
            let color = if is_thumb { Color::Grey } else { Color::DarkGrey };
            queue!(
                stdout,
                cursor::MoveTo((width - 1) as u16, row),
                SetForegroundColor(color),
                Print(ch),
                SetForegroundColor(Color::Reset)
            )?;
        }
    }

    // --- Footer ---
    let footer_row = height.saturating_sub(1);
    queue!(stdout, cursor::MoveTo(0, footer_row))?;
    let (footer_color, footer_text): (Color, String) = match &app.mode {
        Mode::Normal => {
            if let Some((kind, text)) = &app.status {
                let color = match kind {
                    StatusKind::Success => Color::Green,
                    StatusKind::Error => Color::Red,
                    StatusKind::Info => Color::Yellow,
                };
                (color, text.clone())
            } else {
                (
                    Color::DarkGrey,
                    "↑↓:nav  Enter:open  Bksp:up  /:find  ^F:find-all  r:rename  a:new-file  m:mkdir  x:del  .:hidden  q:quit"
                        .to_string(),
                )
            }
        }
        Mode::Search(buf) => (Color::Yellow, format!("find: {buf}▏")),
        Mode::SearchRecursive(buf) => (Color::Yellow, format!("find-all: {buf}▏")),
        Mode::Rename(buf) => (Color::Cyan, format!("rename to: {buf}▏")),
        Mode::NewFile(buf) => (Color::Cyan, format!("new file name: {buf}▏")),
        Mode::NewDir(buf) => (Color::Cyan, format!("new directory name: {buf}▏")),
        Mode::ConfirmDelete => {
            let name = app.selected_entry_name().unwrap_or_default();
            (Color::Red, format!("delete '{name}'? (y/n)"))
        }
    };
    queue!(stdout, SetForegroundColor(footer_color))?;
    write!(stdout, "{}", truncate_to(&footer_text, width))?;
    queue!(stdout, SetForegroundColor(Color::Reset))?;

    stdout.flush()?;
    Ok(())
}
