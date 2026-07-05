use std::fs;
use std::path::Path;

pub struct Config {
    pub ignore_patterns: Vec<String>,
}

impl Config {
    pub fn load(start_path: &Path) -> Self {
        let mut patterns = Vec::new();

        let gitignore_path = start_path.join(".gitignore");
        if gitignore_path.exists() {
            if let Ok(content) = fs::read_to_string(&gitignore_path) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() && !trimmed.starts_with('#') {
                        patterns.push(trimmed.to_string());
                    }
                }
            }
        }

        Config { ignore_patterns: patterns }
    }

    pub fn should_ignore(&self, name: &str, is_dir: bool) -> bool {
        self.ignore_patterns
            .iter()
            .any(|p| pattern_match(name, p, is_dir))
    }
}

fn pattern_match(name: &str, pattern: &str, is_dir: bool) -> bool {
    let pattern = pattern.trim();
    if pattern.is_empty() || pattern.starts_with('#') {
        return false;
    }

    if pattern.starts_with('!') {
        return false;
    }

    let dir_only = pattern.ends_with('/');
    let pat = pattern.trim_end_matches('/');

    if dir_only && !is_dir {
        return false;
    }

    let target = if pat.contains('/') {
        name
    } else {
        name.rsplit('/').next().unwrap_or(name)
    };

    glob_match(target, pat)
}

fn glob_match(s: &str, pat: &str) -> bool {
    let s_chars: Vec<char> = s.chars().collect();
    let p_chars: Vec<char> = pat.chars().collect();
    let mut si = 0;
    let mut pi = 0;
    let mut wild = false;
    let mut backtrack_s = 0;
    let mut backtrack_p = 0;

    while si < s_chars.len() {
        if pi < p_chars.len() && (p_chars[pi] == '?' || p_chars[pi] == s_chars[si]) {
            si += 1;
            pi += 1;
        } else if pi < p_chars.len() && p_chars[pi] == '*' {
            if pi + 1 < p_chars.len() && p_chars[pi + 1] == '*' {
                pi += 2;
                if pi >= p_chars.len() {
                    return true;
                }
                while si < s_chars.len() && s_chars[si] != '/' {
                    si += 1;
                }
                wild = true;
                backtrack_s = si;
                backtrack_p = pi;
            } else {
                wild = true;
                backtrack_p = pi;
                backtrack_s = si + 1;
                pi += 1;
            }
        } else if wild {
            si = backtrack_s;
            pi = backtrack_p;
            backtrack_s += 1;
        } else {
            return false;
        }
    }

    while pi < p_chars.len() && p_chars[pi] == '*' {
        if pi + 1 < p_chars.len() && p_chars[pi + 1] == '*' {
            pi += 2;
        } else {
            pi += 1;
        }
    }

    pi >= p_chars.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match_literal() {
        assert!(glob_match("target", "target"));
        assert!(!glob_match("target", "Target"));
    }

    #[test]
    fn test_glob_match_wildcard() {
        assert!(glob_match("foo.txt", "*.txt"));
        assert!(!glob_match("foo.rs", "*.txt"));
        assert!(glob_match("abc", "*"));
    }

    #[test]
    fn test_glob_match_directory_only() {
        assert!(pattern_match("node_modules", "node_modules/", true));
        assert!(!pattern_match("node_modules", "node_modules/", false));
        assert!(pattern_match("node_modules", "node_modules", true));
    }

    #[test]
    fn test_config_ignore() {
        let config = Config {
            ignore_patterns: vec!["target".to_string(), "*.log".to_string()],
        };
        assert!(config.should_ignore("target", true));
        assert!(config.should_ignore("debug.log", false));
        assert!(config.should_ignore("target", false));
        assert!(!config.should_ignore("src", true));
        assert!(!config.should_ignore("main.rs", false));
    }
}
