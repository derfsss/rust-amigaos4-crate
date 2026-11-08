//! Shared by the SDK constant tests: find the SDK headers and evaluate
//! the simple constant expressions they use.

#![allow(dead_code)]

use std::path::PathBuf;

pub fn include_dir() -> Option<PathBuf> {
    let root = std::env::var("AMIGA_SDK")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sdk"));
    let inc = root.join("include").join("include_h");
    if inc.is_dir() { Some(inc) } else { None }
}

/// The text a header gives `name`: `#define NAME text` or `NAME = text,`.
pub fn definition<'a>(texts: &[&'a str], name: &str) -> Option<&'a str> {
    for text in texts {
        for line in text.lines() {
            let l = line.trim();
            let rest = if let Some(r) = l.strip_prefix("#define") {
                let r = r.trim_start();
                r.strip_prefix(name).filter(|r| r.starts_with(char::is_whitespace))
            } else {
                l.strip_prefix(name)
                    .filter(|r| r.trim_start().starts_with('='))
                    .map(|r| r.trim_start().trim_start_matches('='))
            };
            if let Some(r) = rest {
                let r = r.split("/*").next().unwrap_or("");
                let r = r.split("//").next().unwrap_or("").trim().trim_end_matches(',').trim();
                if !r.is_empty() {
                    return Some(r);
                }
            }
        }
    }
    None
}

/// Evaluate a header expression: integers (decimal, 0x hex, U/L
/// suffixes), names (resolved through `definition`), `|`, `+`, `<<`,
/// parens.
pub fn eval(texts: &[&str], expr: &str, depth: u32) -> Option<i64> {
    if depth > 16 {
        return None;
    }
    let e = expr.trim();
    // Strip one pair of enclosing parentheses.
    if e.starts_with('(') && e.ends_with(')') {
        let inner = &e[1..e.len() - 1];
        let mut d = 0i32;
        let balanced = inner.chars().all(|c| {
            if c == '(' { d += 1 } else if c == ')' { d -= 1 }
            d >= 0
        });
        if balanced && d == 0 {
            return eval(texts, inner, depth + 1);
        }
    }
    // Split at the last top-level operator (left-associative).
    for op in ["|", "+", "<<"] {
        let mut d = 0i32;
        let bytes = e.as_bytes();
        let mut i = bytes.len();
        while i > 0 {
            i -= 1;
            match bytes[i] {
                b')' => d += 1,
                b'(' => d -= 1,
                _ => {}
            }
            if d == 0 && e[i..].starts_with(op) && i > 0 {
                let (a, b) = (&e[..i], &e[i + op.len()..]);
                let (x, y) = (eval(texts, a, depth + 1)?, eval(texts, b, depth + 1)?);
                return Some(match op {
                    "|" => x | y,
                    "+" => x + y,
                    _ => x << y,
                });
            }
        }
    }
    let num = e.trim_end_matches(|c| c == 'U' || c == 'L' || c == 'u' || c == 'l');
    if let Some(h) = num.strip_prefix("0x").or_else(|| num.strip_prefix("0X")) {
        return i64::from_str_radix(h, 16).ok();
    }
    if let Ok(v) = num.parse::<i64>() {
        return Some(v);
    }
    if e.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return eval(texts, definition(texts, e)?, depth + 1);
    }
    None
}
