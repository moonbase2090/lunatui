// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;

use super::chars::{normalize_label, unescape};

const MAX_LABEL_CHARS: usize = 999;
const MAX_PAREN_DEPTH: usize = 32;

#[derive(Debug, Default)]
pub(super) struct Refs {
    map: BTreeMap<String, (String, String)>,
}

impl Refs {
    pub(super) fn insert(&mut self, label: &str, dest: String, title: String) {
        self.map
            .entry(normalize_label(label))
            .or_insert((dest, title));
    }

    pub(super) fn get(&self, label: &str) -> Option<&(String, String)> {
        self.map.get(&normalize_label(label))
    }
}

pub(super) struct Definition {
    pub(super) label: String,
    pub(super) dest: String,
    pub(super) title: String,
    pub(super) consumed: usize,
}

pub(super) fn skip_blank(text: &str, mut pos: usize) -> usize {
    let bytes = text.as_bytes();
    while pos < bytes.len() && matches!(bytes[pos], b' ' | b'\t') {
        pos += 1;
    }
    pos
}

pub(super) fn skip_space_and_one_newline(text: &str, pos: usize) -> usize {
    let bytes = text.as_bytes();
    let mut pos = skip_blank(text, pos);
    if pos < bytes.len() && bytes[pos] == b'\n' {
        pos = skip_blank(text, pos + 1);
    }
    pos
}

pub(super) fn parse_label(text: &str, pos: usize) -> Option<(&str, usize)> {
    let bytes = text.as_bytes();
    if bytes.get(pos) != Some(&b'[') {
        return None;
    }
    let start = pos + 1;
    let mut index = start;
    let mut has_content = false;
    while index < bytes.len() {
        if index - start > MAX_LABEL_CHARS * 4 {
            return None;
        }
        match bytes[index] {
            b'\\' if bytes.get(index + 1).is_some_and(u8::is_ascii_punctuation) => {
                has_content = true;
                index += 2;
                continue;
            }
            b'[' => return None,
            b']' => {
                let inner = &text[start..index];
                if !has_content || inner.chars().count() > MAX_LABEL_CHARS {
                    return None;
                }
                return Some((inner, index + 1));
            }
            b' ' | b'\t' | b'\n' | b'\r' | 0x0c => {}
            _ => has_content = true,
        }
        index += 1;
    }
    None
}

pub(super) fn parse_destination(text: &str, pos: usize) -> Option<(String, usize, bool)> {
    let bytes = text.as_bytes();
    if bytes.get(pos) == Some(&b'<') {
        let start = pos + 1;
        let mut index = start;
        while index < bytes.len() {
            match bytes[index] {
                b'\\' if bytes.get(index + 1).is_some_and(u8::is_ascii_punctuation) => {
                    index += 2;
                    continue;
                }
                b'\n' | b'<' => return None,
                b'>' => return Some((unescape(&text[start..index]), index + 1, true)),
                _ => {}
            }
            index += 1;
        }
        return None;
    }
    let mut depth = 0usize;
    let mut index = pos;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' if bytes.get(index + 1).is_some_and(u8::is_ascii_punctuation) => {
                index += 2;
                continue;
            }
            b'(' => {
                depth += 1;
                if depth > MAX_PAREN_DEPTH {
                    return None;
                }
            }
            b')' => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            b' ' | 0..=0x1f | 0x7f => break,
            _ => {}
        }
        index += 1;
    }
    if depth != 0 {
        return None;
    }
    Some((unescape(&text[pos..index]), index, false))
}

pub(super) fn parse_title(text: &str, pos: usize) -> Option<(String, usize)> {
    let bytes = text.as_bytes();
    let open = *bytes.get(pos)?;
    let close = match open {
        b'"' => b'"',
        b'\'' => b'\'',
        b'(' => b')',
        _ => return None,
    };
    let start = pos + 1;
    let mut index = start;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'\\' && bytes.get(index + 1).is_some_and(u8::is_ascii_punctuation) {
            index += 2;
            continue;
        }
        if byte == close {
            return Some((unescape(&text[start..index]), index + 1));
        }
        if open == b'(' && byte == b'(' {
            return None;
        }
        index += 1;
    }
    None
}

fn line_rest_is_blank(text: &str, pos: usize) -> Option<usize> {
    let rest = &text[pos..];
    let end = rest.find('\n').unwrap_or(rest.len());
    if rest[..end].bytes().all(|b| matches!(b, b' ' | b'\t')) {
        Some(pos + end + usize::from(end < rest.len()))
    } else {
        None
    }
}

pub(super) fn parse_definition(text: &str) -> Option<Definition> {
    let start = skip_blank(text, 0);
    let (label, after_label) = parse_label(text, start)?;
    if text.as_bytes().get(after_label) != Some(&b':') {
        return None;
    }
    if normalize_label(label).is_empty() {
        return None;
    }
    let dest_pos = skip_space_and_one_newline(text, after_label + 1);
    let (dest, after_dest, angle) = parse_destination(text, dest_pos)?;
    if dest.is_empty() && !angle {
        return None;
    }
    let title_pos = skip_space_and_one_newline(text, after_dest);
    if title_pos > after_dest
        && let Some((title, after_title)) = parse_title(text, title_pos)
        && let Some(consumed) = line_rest_is_blank(text, after_title)
    {
        return Some(Definition {
            label: String::from(label),
            dest,
            title,
            consumed,
        });
    }
    let consumed = line_rest_is_blank(text, after_dest)?;
    Some(Definition {
        label: String::from(label),
        dest,
        title: String::new(),
        consumed,
    })
}
