// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use super::ast::Alignment;

pub(super) fn split_row(line: &str) -> Vec<String> {
    let trimmed = line.trim_matches([' ', '\t']);
    let body = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    let mut closed_by_pipe = false;
    for c in body.chars() {
        closed_by_pipe = false;
        if escaped {
            if c != '|' {
                current.push('\\');
            }
            current.push(c);
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '|' {
            cells.push(String::from(current.trim_matches([' ', '\t'])));
            current.clear();
            closed_by_pipe = true;
        } else {
            current.push(c);
        }
    }
    if escaped {
        current.push('\\');
    }
    let last = String::from(current.trim_matches([' ', '\t']));
    if !(closed_by_pipe && last.is_empty()) {
        cells.push(last);
    }
    cells
}

pub(super) fn delimiter_row(line: &str) -> Option<Vec<Alignment>> {
    let trimmed = line.trim_matches([' ', '\t']);
    if !trimmed.contains('|') {
        return None;
    }
    let mut alignments = Vec::new();
    for cell in split_row(trimmed) {
        let left = cell.starts_with(':');
        let right = cell.ends_with(':');
        let dashes = cell.trim_matches(':');
        if dashes.is_empty() || !dashes.bytes().all(|byte| byte == b'-') {
            return None;
        }
        alignments.push(match (left, right) {
            (true, true) => Alignment::Center,
            (true, false) => Alignment::Left,
            (false, true) => Alignment::Right,
            (false, false) => Alignment::None,
        });
    }
    (!alignments.is_empty()).then_some(alignments)
}
