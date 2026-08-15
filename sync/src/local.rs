//! Reading and writing langbank's own data files.
//!
//! Deliberately textual. These files are hand-edited as often as generated, and
//! round-tripping them through a TOML serialiser would reformat comments and
//! ordering that a person put there on purpose. So a value is read with a
//! narrow parse and written by splicing one line, and everything else in the
//! file is left exactly as it was found.

use std::path::{Path, PathBuf};

use crate::report::Result;

/// Every `*.toml` in a directory, sorted.
///
/// Sorted because it once was not: an index built with `glob` in filesystem
/// order made a coverage check pass locally and fail on a runner over identical
/// data, which took a while to believe.
pub fn files(directory: &str) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !Path::new(directory).is_dir() {
        return Ok(out);
    }
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "toml")
        {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

/// A bare `key = "value"` at the top level.
pub fn scalar(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let Some(rest) = line.strip_prefix(key) else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix('=') else {
            continue;
        };
        let rest = rest.trim();
        if let Some(value) = rest.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
            return Some(unescape(value));
        }
    }
    None
}

// Used by the sources still being ported; they land one at a time.
#[allow(dead_code)]
/// A top-level `key = [...]`, which may span lines.
pub fn array(text: &str, key: &str) -> Vec<String> {
    let mut out = Vec::new();
    let Some(start) = find_key(text, key) else {
        return out;
    };
    let after = &text[start..];
    let Some(open) = after.find('[') else {
        return out;
    };
    let Some(close) = after[open..].find(']') else {
        return out;
    };
    let mut chars = after[open + 1..open + close].chars().peekable();
    let mut current = String::new();
    let mut inside = false;
    while let Some(c) = chars.next() {
        match c {
            '"' if !inside => inside = true,
            '"' => {
                inside = false;
                out.push(std::mem::take(&mut current));
            }
            '\\' if inside => {
                if let Some(next) = chars.next() {
                    current.push(match next {
                        'n' => '\n',
                        't' => '\t',
                        other => other,
                    });
                }
            }
            _ if inside => current.push(c),
            _ => {}
        }
    }
    out
}

#[allow(dead_code)]
fn find_key(text: &str, key: &str) -> Option<usize> {
    let mut offset = 0;
    for line in text.lines() {
        if line.starts_with(key) && line[key.len()..].trim_start().starts_with('=') {
            return Some(offset);
        }
        offset += line.len() + 1;
    }
    None
}

fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

/// TOML takes literal UTF-8. A JSON encoder that escapes non-BMP characters
/// into surrogate pairs produces something TOML rejects, which is how Mojo's
/// `.🔥` extension broke a build once.
pub fn toml_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

#[allow(dead_code)]
pub fn toml_array<S: AsRef<str>>(values: &[S]) -> String {
    let inner = values
        .iter()
        .map(|value| toml_string(value.as_ref()))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{inner}]")
}

#[allow(dead_code)]
/// Replace a top-level key's line, or insert one after `after` if absent.
pub fn upsert_line(text: &str, key: &str, line: &str, after: &str) -> String {
    if let Some(start) = find_key(text, key) {
        let rest = &text[start..];
        let end = rest
            .find(']')
            .map(|bracket| start + bracket + 1)
            .filter(|_| rest.starts_with(&format!("{key} = [")))
            .unwrap_or_else(|| start + rest.find('\n').unwrap_or(rest.len()));
        let tail = text[end..].strip_prefix('\n').unwrap_or(&text[end..]);
        return format!("{}{line}\n{tail}", &text[..start]);
    }
    match find_key(text, after) {
        Some(start) => {
            let rest = &text[start..];
            let end = start + rest.find('\n').map_or(rest.len(), |n| n + 1);
            format!("{}{line}\n{}", &text[..end], &text[end..])
        }
        None => format!("{text}{line}\n"),
    }
}
