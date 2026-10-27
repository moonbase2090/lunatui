// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use super::ast::Inline;
use super::chars::is_space;

pub(super) fn apply(inlines: Vec<Inline>) -> Vec<Inline> {
    let mut out = Vec::with_capacity(inlines.len());
    let mut line_start = true;
    for node in inlines {
        match node {
            Inline::Text(text) => {
                out.extend(expand(&text, line_start));
                line_start = false;
            }
            Inline::Emphasis(content) => {
                out.push(Inline::Emphasis(apply(content)));
                line_start = false;
            }
            Inline::Strong(content) => {
                out.push(Inline::Strong(apply(content)));
                line_start = false;
            }
            Inline::Strikethrough(content) => {
                out.push(Inline::Strikethrough(apply(content)));
                line_start = false;
            }
            Inline::SoftBreak | Inline::HardBreak => {
                out.push(node);
                line_start = true;
            }
            other => {
                out.push(other);
                line_start = false;
            }
        }
    }
    out
}

fn push_text(out: &mut Vec<Inline>, piece: &str) {
    if piece.is_empty() {
        return;
    }
    if let Some(Inline::Text(existing)) = out.last_mut() {
        existing.push_str(piece);
    } else {
        out.push(Inline::Text(String::from(piece)));
    }
}

fn boundary_ok(text: &str, at: usize, line_start: bool) -> bool {
    match text[..at].chars().next_back() {
        None => line_start,
        Some(c) => is_space(c) || matches!(c, '*' | '_' | '~' | '('),
    }
}

fn expand(text: &str, line_start: bool) -> Vec<Inline> {
    let mut out = Vec::new();
    let mut plain_from = 0;
    let mut index = 0;
    while index < text.len() {
        let Some(c) = text[index..].chars().next() else {
            break;
        };
        let after_local = text[..index].chars().next_back().is_some_and(is_local);
        let found = if boundary_ok(text, index, line_start) {
            url_at(text, index).or_else(|| email_at(text, index).filter(|_| !after_local))
        } else {
            email_at(text, index).filter(|_| !after_local)
        };
        if let Some((end, dest)) = found {
            push_text(&mut out, &text[plain_from..index]);
            out.push(Inline::Link {
                dest,
                title: String::new(),
                content: alloc::vec![Inline::Text(String::from(&text[index..end]))],
            });
            index = end;
            plain_from = end;
            continue;
        }
        index += c.len_utf8();
    }
    push_text(&mut out, &text[plain_from..]);
    out
}

fn starts_with_ci(text: &str, prefix: &str) -> bool {
    text.len() >= prefix.len()
        && text.as_bytes()[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes())
}

fn domain_end(text: &str, from: usize) -> usize {
    let mut end = from;
    for (offset, c) in text[from..].char_indices() {
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
            end = from + offset + c.len_utf8();
        } else {
            break;
        }
    }
    end
}

fn domain_valid(domain: &str) -> bool {
    let trimmed = domain.trim_end_matches('.');
    let segments: Vec<&str> = trimmed.split('.').collect();
    if segments.len() < 2 || segments.iter().any(|segment| segment.is_empty()) {
        return false;
    }
    segments
        .iter()
        .rev()
        .take(2)
        .all(|segment| !segment.contains('_'))
}

fn url_at(text: &str, index: usize) -> Option<(usize, String)> {
    let rest = &text[index..];
    let (domain_start, prefix) = if starts_with_ci(rest, "www.") {
        (index, "http://")
    } else if starts_with_ci(rest, "http://") {
        (index + 7, "")
    } else if starts_with_ci(rest, "https://") {
        (index + 8, "")
    } else if starts_with_ci(rest, "ftp://") {
        (index + 6, "")
    } else {
        return None;
    };
    let domain_stop = domain_end(text, domain_start);
    if !domain_valid(&text[domain_start..domain_stop]) {
        return None;
    }
    let mut end = domain_stop;
    for (offset, c) in text[domain_stop..].char_indices() {
        if is_space(c) || c == '<' {
            break;
        }
        end = domain_stop + offset + c.len_utf8();
    }
    let end = trim_trailing(text, index, end);
    let mut dest = String::from(prefix);
    dest.push_str(&text[index..end]);
    Some((end, dest))
}

fn trim_trailing(text: &str, start: usize, mut end: usize) -> usize {
    loop {
        let candidate = &text[start..end];
        let Some(last) = candidate.chars().next_back() else {
            return end;
        };
        match last {
            '?' | '!' | '.' | ',' | ':' | '*' | '_' | '~' => end -= 1,
            ')' => {
                let opens = candidate.matches('(').count();
                let closes = candidate.matches(')').count();
                if closes > opens {
                    end -= 1;
                } else {
                    return end;
                }
            }
            ';' => {
                let body = &candidate[..candidate.len() - 1];
                let name_len = body
                    .bytes()
                    .rev()
                    .take_while(u8::is_ascii_alphanumeric)
                    .count();
                if name_len > 0 && body[..body.len() - name_len].ends_with('&') {
                    end -= name_len + 2;
                } else {
                    return end;
                }
            }
            _ => return end,
        }
    }
}

fn is_local(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+')
}

fn email_at(text: &str, index: usize) -> Option<(usize, String)> {
    let rest = &text[index..];
    let local_len: usize = rest
        .chars()
        .take_while(|&c| is_local(c))
        .map(char::len_utf8)
        .sum();
    if local_len == 0 || rest.as_bytes().get(local_len) != Some(&b'@') {
        return None;
    }
    let domain_start = index + local_len + 1;
    let stop = domain_end(text, domain_start);
    let domain = text[domain_start..stop].trim_end_matches('.');
    if domain.is_empty() || !domain.contains('.') || domain.ends_with(['-', '_']) {
        return None;
    }
    if domain.split('.').any(str::is_empty) {
        return None;
    }
    let end = domain_start + domain.len();
    let mut dest = String::from("mailto:");
    dest.push_str(&text[index..end]);
    Some((end, dest))
}
