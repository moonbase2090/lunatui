// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::char_width;

use super::canvas::text_columns;

pub(super) const MAX_LINES: usize = 8;
pub(super) const MAX_COLUMNS: i32 = 100;

fn break_at(chars: &[char], at: usize) -> Option<usize> {
    let tag = |offset: usize| chars.get(at + offset).map(char::to_ascii_lowercase);
    if chars.get(at) != Some(&'<') || tag(1) != Some('b') || tag(2) != Some('r') {
        return None;
    }
    let mut next = at + 3;
    while chars.get(next) == Some(&' ') {
        next += 1;
    }
    if chars.get(next) == Some(&'/') {
        next += 1;
    }
    (chars.get(next) == Some(&'>')).then_some(next + 1)
}

pub(super) fn clean(raw: &str) -> Result<Vec<String>, &'static str> {
    let chars: Vec<char> = raw.chars().collect();
    let mut lines = vec![String::new()];
    let mut at = 0;
    while let Some(&ch) = chars.get(at) {
        if let Some(next) = break_at(&chars, at) {
            lines.push(String::new());
            at = next;
            continue;
        }
        at += 1;
        let keep = ch == '\t' || (!ch.is_control() && char_width(ch) > 0);
        if let (true, Some(line)) = (keep, lines.last_mut()) {
            line.push(if ch == '\t' { ' ' } else { ch });
        }
    }
    let lines: Vec<String> = lines
        .into_iter()
        .map(|line| line.trim().to_owned())
        .filter(|line| !line.is_empty())
        .collect();
    if lines.len() > MAX_LINES {
        return Err("label has too many lines");
    }
    if lines.iter().any(|line| text_columns(line) > MAX_COLUMNS) {
        return Err("label is too wide");
    }
    Ok(lines)
}

pub(super) fn single_line(raw: &str) -> Result<String, &'static str> {
    Ok(clean(raw)?.join(" "))
}
