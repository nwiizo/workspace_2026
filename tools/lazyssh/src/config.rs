use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Host {
    pub alias: String,
    pub host_name: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub identity_files: Vec<String>,
    pub proxy_jump: Option<String>,
    pub source: PathBuf,
    pub line: usize,
}

pub fn load(path: &Path) -> io::Result<Vec<Host>> {
    let include_base = home_dir().map(|home| home.join(".ssh")).unwrap_or_else(|| {
        path.parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    });
    let mut parser = Parser::new(include_base);
    parser.parse_file(path)?;
    Ok(parser.hosts)
}

struct Parser {
    hosts: Vec<Host>,
    host_indices: HashMap<String, usize>,
    active_hosts: Vec<usize>,
    visiting: HashSet<PathBuf>,
    include_base: PathBuf,
}

impl Parser {
    fn new(include_base: PathBuf) -> Self {
        Self {
            hosts: Vec::new(),
            host_indices: HashMap::new(),
            active_hosts: Vec::new(),
            visiting: HashSet::new(),
            include_base,
        }
    }

    fn parse_file(&mut self, path: &Path) -> io::Result<()> {
        let path = fs::canonicalize(path)?;
        if !self.visiting.insert(path.clone()) {
            return Ok(());
        }

        let result = self.parse_contents(&path);
        self.visiting.remove(&path);
        result
    }

    fn parse_contents(&mut self, path: &Path) -> io::Result<()> {
        let contents = fs::read_to_string(path)?;
        for (line_index, line) in contents.lines().enumerate() {
            let Some((keyword, arguments)) = parse_directive(line) else {
                continue;
            };

            match keyword.as_str() {
                "host" => self.begin_host_block(arguments, path, line_index + 1),
                "match" => self.active_hosts.clear(),
                "include" => self.parse_includes(arguments)?,
                _ => self.apply_option(&keyword, &arguments),
            }
        }
        Ok(())
    }

    fn begin_host_block(&mut self, aliases: Vec<String>, source: &Path, line: usize) {
        self.active_hosts.clear();
        for alias in aliases {
            if alias.is_empty() || alias.starts_with(['!', '-']) || alias.contains(['*', '?']) {
                continue;
            }

            let index = if let Some(index) = self.host_indices.get(&alias) {
                *index
            } else {
                let index = self.hosts.len();
                self.hosts.push(Host {
                    alias: alias.clone(),
                    host_name: None,
                    user: None,
                    port: None,
                    identity_files: Vec::new(),
                    proxy_jump: None,
                    source: source.to_path_buf(),
                    line,
                });
                self.host_indices.insert(alias, index);
                index
            };

            if !self.active_hosts.contains(&index) {
                self.active_hosts.push(index);
            }
        }
    }

    fn parse_includes(&mut self, patterns: Vec<String>) -> io::Result<()> {
        for pattern in patterns {
            let pattern = resolve_include_pattern(&pattern, &self.include_base);
            let Some(pattern) = pattern.to_str() else {
                continue;
            };

            let paths = glob::glob(pattern).map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("invalid Include pattern {pattern:?}: {error}"),
                )
            })?;
            let mut paths = paths.filter_map(Result::ok).collect::<Vec<_>>();
            paths.sort();
            for path in paths {
                self.parse_file(&path)?;
            }
        }
        Ok(())
    }

    fn apply_option(&mut self, keyword: &str, arguments: &[String]) {
        let Some(value) = arguments.first() else {
            return;
        };

        for index in self.active_hosts.iter().copied() {
            let host = &mut self.hosts[index];
            match keyword {
                "hostname" if host.host_name.is_none() => host.host_name = Some(value.clone()),
                "user" if host.user.is_none() => host.user = Some(value.clone()),
                "port" if host.port.is_none() => host.port = value.parse().ok(),
                "identityfile" => host.identity_files.push(value.clone()),
                "proxyjump" if host.proxy_jump.is_none() => {
                    host.proxy_jump = Some(value.clone());
                }
                _ => {}
            }
        }
    }
}

fn resolve_include_pattern(pattern: &str, include_base: &Path) -> PathBuf {
    if pattern == "~" {
        return home_dir().unwrap_or_else(|| PathBuf::from(pattern));
    }
    if let Some(rest) = pattern.strip_prefix("~/")
        && let Some(home) = home_dir()
    {
        return home.join(rest);
    }

    let path = Path::new(pattern);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        include_base.join(path)
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn parse_directive(line: &str) -> Option<(String, Vec<String>)> {
    let mut tokens = tokenize(line);
    if tokens.is_empty() {
        return None;
    }

    let first = tokens.remove(0);
    let (keyword, first_value) = first
        .split_once('=')
        .map_or((first.as_str(), None), |(key, value)| (key, Some(value)));
    if let Some(value) = first_value.filter(|value| !value.is_empty()) {
        tokens.insert(0, value.to_owned());
    } else if tokens.first().is_some_and(|token| token == "=") {
        tokens.remove(0);
    }
    Some((keyword.to_ascii_lowercase(), tokens))
}

fn tokenize(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;

    for character in line.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }
        if character == '\\' {
            escaped = true;
            continue;
        }
        if let Some(delimiter) = quote {
            if character == delimiter {
                quote = None;
            } else {
                current.push(character);
            }
            continue;
        }
        if matches!(character, '\'' | '"') {
            quote = Some(character);
        } else if character == '#' {
            break;
        } else if character.is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
        } else {
            current.push(character);
        }
    }

    if escaped {
        current.push('\\');
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}
