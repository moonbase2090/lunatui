// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

extern crate alloc;

use alloc::string::String;

use super::tables::{ENTITIES, PUNCTUATION, SPACE};

fn in_ranges(table: &[(u32, u32)], value: u32) -> bool {
    table
        .binary_search_by(|&(lo, hi)| {
            if value < lo {
                core::cmp::Ordering::Greater
            } else if value > hi {
                core::cmp::Ordering::Less
            } else {
                core::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

pub(super) fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\x0c' | '\r')
        || (!c.is_ascii() && in_ranges(&SPACE, u32::from(c)))
}

pub(super) fn is_punct(c: char) -> bool {
    if c.is_ascii() {
        c.is_ascii_punctuation()
    } else {
        in_ranges(&PUNCTUATION, u32::from(c))
    }
}

pub(super) fn entity_text(name: &str) -> Option<&'static str> {
    let index = ENTITIES
        .binary_search_by(|&(entry, _)| entry.as_bytes().cmp(name.as_bytes()))
        .ok()?;
    Some(ENTITIES[index].1)
}

pub(super) fn parse_entity(text: &str) -> Option<(String, usize)> {
    let rest = text.strip_prefix('&')?;
    let end = rest.bytes().take(34).position(|b| b == b';')?;
    let body = &rest[..end];
    let consumed = end + 2;
    if let Some(number) = body.strip_prefix('#') {
        let (digits, radix, max_digits) = match number.strip_prefix(['x', 'X']) {
            Some(hex) => (hex, 16, 6),
            None => (number, 10, 7),
        };
        if digits.is_empty()
            || digits.len() > max_digits
            || !digits.chars().all(|c| c.is_digit(radix))
        {
            return None;
        }
        let value = u32::from_str_radix(digits, radix).ok()?;
        let decoded = match char::from_u32(value) {
            Some(c) if value != 0 => c,
            _ => '\u{FFFD}',
        };
        return Some((String::from(decoded), consumed));
    }
    if body.is_empty() || !body.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return None;
    }
    entity_text(body).map(|text| (String::from(text), consumed))
}

pub(super) fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if c == '\\' {
            let after = &rest[1..];
            if let Some(next) = after.chars().next()
                && next.is_ascii_punctuation()
            {
                out.push(next);
                rest = &after[next.len_utf8()..];
                continue;
            }
        } else if c == '&'
            && let Some((decoded, used)) = parse_entity(rest)
        {
            out.push_str(&decoded);
            rest = &rest[used..];
            continue;
        }
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    out
}

pub(super) fn normalize_label(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    let mut pending_space = false;
    for c in label.chars() {
        if is_space(c) {
            pending_space = !out.is_empty();
            continue;
        }
        if pending_space {
            out.push(' ');
            pending_space = false;
        }
        if c == '\u{1E9E}' {
            out.push_str("ss");
            continue;
        }
        for upper in c.to_uppercase() {
            out.extend(upper.to_lowercase());
        }
    }
    out
}
