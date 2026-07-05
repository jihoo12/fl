use std::env;
use std::fs;
use std::io;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = if args.len() > 1 {
        &args[1]
    } else {
        "."
    };

    let root = Path::new(path);
    if !root.is_dir() {
        eprintln!("error: '{}' is not a directory", path);
        std::process::exit(1);
    }

    println!("{}", root.display());
    if let Err(e) = tree(root, "") {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}

fn tree(dir: &Path, prefix: &str) -> io::Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let last = entries.len().saturating_sub(1);
    for (i, entry) in entries.iter().enumerate() {
        let name = entry.file_name();
        let is_dir = entry.file_type()?.is_dir();
        let connector = if i == last { "└── " } else { "├── " };
        println!("{}{}{}", prefix, connector, name.to_string_lossy());

        if is_dir {
            let new_prefix = format!("{}{}", prefix, if i == last { "    " } else { "│   " });
            tree(&entry.path(), &new_prefix)?;
        }
    }
    Ok(())
}
