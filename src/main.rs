mod browser;
mod config;
mod entry;
mod search;
mod ui;

use std::env;
use std::error::Error;
use std::io;
use std::path::PathBuf;

use crossterm::{
    cursor, execute,
    terminal::{self},
};

use crate::browser::Browser;
use crate::config::Config;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let initial_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        env::current_dir()?
    };

    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        terminal::DisableLineWrap,
        cursor::Hide
    )?;

    let config = Config::load(&initial_path);
    let mut app = Browser::new(initial_path, config);
    let res = browser::run_browser(&mut app);

    execute!(
        stdout,
        terminal::EnableLineWrap,
        terminal::LeaveAlternateScreen,
        cursor::Show
    )?;
    terminal::disable_raw_mode()?;

    res
}
