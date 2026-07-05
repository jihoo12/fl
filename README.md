# fl — Terminal File Explorer

A keyboard-driven terminal file browser written in Rust.

## Installation

```bash
# Nix
nix run github:jihoo12/fl
nix shell github:user/fl

# Cargo
cargo install --git https://github.com/jihoo12/fl
```

## Usage

```bash
fl [path]
```

### Keybindings

| Key | Action |
|---|---|
| `↑`/`↓` or `k`/`j` | Navigate |
| `Enter` | Open file / enter directory |
| `Backspace` | Go to parent directory |
| `g` / `G` | Go to top / bottom |
| `Home` / `End` | Go to top / bottom |
| `PageUp` / `PageDown` | Scroll page |
| `/` | Filter entries by prefix |
| `Ctrl+f` | Recursive search across subdirectories |
| `r` | Rename selected entry |
| `a` | Create new file |
| `m` | Create new directory |
| `x` or `Delete` | Delete selected entry |
| `.` | Toggle hidden files |
| `q` or `Esc` | Quit |

### Recursive search (`Ctrl+f`)

Type a query and press `Enter` to search all subdirectories. Results show relative paths. Navigate with arrow keys and open with `Enter`. Press `Backspace` or `Esc` to return.

## Config

`fl` reads `.gitignore` from the startup directory and ignores matching files/directories. Patterns support `*`, `?`, `**`, and trailing `/` (directory-only).

## From source

```bash
git clone https://github.com/jihoo12/fl
cd fl
cargo build --release
./target/release/fl
```
