// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use super::ast::{Extension, Inline, Options};
use super::chars::{is_punct, is_space, parse_entity};
use super::html::{Ends, autolink_len, inline_html_len};
use super::links::{Refs, parse_destination, parse_label, parse_title, skip_space_and_one_newline};

#[derive(Debug)]
struct Delim {
    ch: u8,
    count: usize,
    orig: usize,
    can_open: bool,
    can_close: bool,
}

#[derive(Debug)]
struct Bracket {
    image: bool,
    src_start: usize,
}

#[derive(Debug)]
enum Tok {
    Node(Inline),
    Delim(Delim),
    Bracket(Bracket),
}

struct Parser<'a> {
    text: &'a str,
    pos: usize,
    toks: Vec<Tok>,
    refs: &'a Refs,
    options: Options,
    failed_ticks: Vec<usize>,
    ends: Ends,
    brackets: Vec<usize>,
    barrier: usize,
}

pub(super) fn parse(text: &str, refs: &Refs, options: Options) -> Vec<Inline> {
    let mut parser = Parser {
        text,
        pos: 0,
        toks: Vec::new(),
        refs,
        options,
        failed_ticks: Vec::new(),
        ends: Ends::new(text),
        brackets: Vec::new(),
        barrier: 0,
    };
    parser.run();
    let toks = core::mem::take(&mut parser.toks);
    let inlines = build(toks);
    if options.enabled(Extension::Autolinks) {
        super::autolink::apply(inlines)
    } else {
        inlines
    }
}

fn is_special(byte: u8, strike: bool) -> bool {
    matches!(
        byte,
        b'\\' | b'`' | b'*' | b'_' | b'[' | b']' | b'!' | b'<' | b'&' | b'\n'
    ) || (strike && byte == b'~')
}

impl Parser<'_> {
    fn run(&mut self) {
        let bytes = self.text.as_bytes();
        while self.pos < bytes.len() {
            match bytes[self.pos] {
                b'\\' => self.escape(),
                b'`' => self.backticks(),
                b'*' | b'_' => self.delimiter_run(),
                b'~' if self.options.enabled(Extension::Strikethrough) => self.delimiter_run(),
                b'[' => {
                    self.open_bracket(false, self.pos + 1);
                    self.pos += 1;
                }
                b'!' if bytes.get(self.pos + 1) == Some(&b'[') => {
                    self.open_bracket(true, self.pos + 2);
                    self.pos += 2;
                }
                b']' => self.close_bracket(),
                b'<' => self.angle(),
                b'&' => self.entity(),
                b'\n' => self.line_break(),
                _ => self.plain_text(),
            }
        }
    }

    fn open_bracket(&mut self, image: bool, src_start: usize) {
        self.brackets.push(self.toks.len());
        self.toks.push(Tok::Bracket(Bracket { image, src_start }));
    }

    fn push_text(&mut self, piece: &str) {
        if piece.is_empty() {
            return;
        }
        if let Some(Tok::Node(Inline::Text(existing))) = self.toks.last_mut() {
            existing.push_str(piece);
        } else {
            self.toks.push(Tok::Node(Inline::Text(String::from(piece))));
        }
    }

    fn plain_text(&mut self) {
        let bytes = self.text.as_bytes();
        let strike = self.options.enabled(Extension::Strikethrough);
        let start = self.pos;
        let mut end = start + 1;
        while end < bytes.len() && !is_special(bytes[end], strike) {
            end += 1;
        }
        let piece = &self.text[start..end];
        self.pos = end;
        self.push_text(piece);
    }

    fn escape(&mut self) {
        let rest = &self.text[self.pos + 1..];
        match rest.chars().next() {
            Some(c) if c.is_ascii_punctuation() => {
                let mut buffer = [0u8; 4];
                self.push_text(c.encode_utf8(&mut buffer));
                self.pos += 2;
            }
            Some('\n') => {
                self.toks.push(Tok::Node(Inline::HardBreak));
                self.pos += 2;
                self.skip_leading_blanks();
            }
            _ => {
                self.push_text("\\");
                self.pos += 1;
            }
        }
    }

    fn skip_leading_blanks(&mut self) {
        let bytes = self.text.as_bytes();
        while self.pos < bytes.len() && matches!(bytes[self.pos], b' ' | b'\t') {
            self.pos += 1;
        }
    }

    fn line_break(&mut self) {
        let mut hard = false;
        if let Some(Tok::Node(Inline::Text(existing))) = self.toks.last_mut() {
            let trimmed = existing.trim_end_matches(' ').len();
            hard = existing.len() - trimmed >= 2;
            existing.truncate(trimmed);
            if existing.is_empty() {
                self.toks.pop();
            }
        }
        self.toks.push(Tok::Node(if hard {
            Inline::HardBreak
        } else {
            Inline::SoftBreak
        }));
        self.pos += 1;
        self.skip_leading_blanks();
    }

    fn entity(&mut self) {
        if let Some((decoded, used)) = parse_entity(&self.text[self.pos..]) {
            self.push_text(&decoded);
            self.pos += used;
        } else {
            self.push_text("&");
            self.pos += 1;
        }
    }

    fn backticks(&mut self) {
        let bytes = self.text.as_bytes();
        let start = self.pos;
        let mut end = start;
        while end < bytes.len() && bytes[end] == b'`' {
            end += 1;
        }
        let length = end - start;
        let close = if self.failed_ticks.contains(&length) {
            None
        } else {
            find_backtick_close(self.text, end, length)
        };
        let Some(close) = close else {
            self.failed_ticks.push(length);
            self.push_text(&self.text[start..end]);
            self.pos = end;
            return;
        };
        let mut content: String = self.text[end..close]
            .chars()
            .map(|c| if c == '\n' { ' ' } else { c })
            .collect();
        if content.len() > 2
            && content.starts_with(' ')
            && content.ends_with(' ')
            && content.chars().any(|c| c != ' ')
        {
            content = String::from(&content[1..content.len() - 1]);
        }
        self.toks.push(Tok::Node(Inline::Code(content)));
        self.pos = close + length;
    }

    fn delimiter_run(&mut self) {
        let bytes = self.text.as_bytes();
        let ch = bytes[self.pos];
        let start = self.pos;
        let mut end = start;
        while end < bytes.len() && bytes[end] == ch {
            end += 1;
        }
        let count = end - start;
        let before = self.text[..start].chars().next_back();
        let after = self.text[end..].chars().next();
        let before_space = before.is_none_or(is_space);
        let after_space = after.is_none_or(is_space);
        let before_punct = before.is_some_and(is_punct);
        let after_punct = after.is_some_and(is_punct);
        let left = !after_space && (!after_punct || before_space || before_punct);
        let right = !before_space && (!before_punct || after_space || after_punct);
        let (can_open, can_close) = match ch {
            b'_' => (
                left && (!right || before_punct),
                right && (!left || after_punct),
            ),
            _ => (left, right),
        };
        let usable = ch != b'~' || count <= 2;
        self.pos = end;
        if (can_open || can_close) && usable {
            self.toks.push(Tok::Delim(Delim {
                ch,
                count,
                orig: count,
                can_open,
                can_close,
            }));
        } else {
            self.push_text(&self.text[start..end]);
        }
    }

    fn angle(&mut self) {
        let rest = &self.text[self.pos..];
        if let Some(length) = autolink_len(rest) {
            let inner = &rest[1..length - 1];
            let dest = if inner.contains(':') {
                String::from(inner)
            } else {
                alloc::format!("mailto:{inner}")
            };
            self.toks.push(Tok::Node(Inline::Link {
                dest,
                title: String::new(),
                content: alloc::vec![Inline::Text(String::from(inner))],
            }));
            self.pos += length;
        } else if let Some(length) = inline_html_len(rest, self.pos, &self.ends) {
            self.toks
                .push(Tok::Node(Inline::Html(String::from(&rest[..length]))));
            self.pos += length;
        } else {
            self.push_text("<");
            self.pos += 1;
        }
    }

    fn close_bracket(&mut self) {
        let close = self.pos;
        let Some(&index) = self.brackets.last() else {
            self.push_text("]");
            self.pos += 1;
            return;
        };
        let (image, src_start) = match &self.toks[index] {
            Tok::Bracket(bracket) => (bracket.image, bracket.src_start),
            _ => return,
        };
        let literal = if image { "![" } else { "[" };
        let active = image || self.brackets.len() > self.barrier;
        let resolved = if active {
            self.inline_link(close + 1)
                .or_else(|| self.reference(src_start, close))
        } else {
            None
        };
        let Some((dest, title, end)) = resolved else {
            self.toks[index] = Tok::Node(Inline::Text(String::from(literal)));
            self.brackets.pop();
            self.barrier = self.barrier.min(self.brackets.len());
            self.push_text("]");
            self.pos += 1;
            return;
        };
        let children: Vec<Tok> = self.toks.split_off(index + 1);
        self.toks.pop();
        while self.brackets.last().is_some_and(|&at| at >= index) {
            self.brackets.pop();
        }
        let content = build(children);
        let node = if image {
            Inline::Image {
                dest,
                title,
                alt: content,
            }
        } else {
            Inline::Link {
                dest,
                title,
                content,
            }
        };
        self.toks.push(Tok::Node(node));
        if image {
            self.barrier = self.barrier.min(self.brackets.len());
        } else {
            self.barrier = self.brackets.len();
        }
        self.pos = end;
    }

    fn inline_link(&self, after: usize) -> Option<(String, String, usize)> {
        let text = self.text;
        if text.as_bytes().get(after) != Some(&b'(') {
            return None;
        }
        let mut pos = skip_space_and_one_newline(text, after + 1);
        let mut dest = String::new();
        let mut title = String::new();
        if text.as_bytes().get(pos) != Some(&b')') {
            let (parsed, end, _) = parse_destination(text, pos)?;
            dest = parsed;
            pos = end;
            let spaced = skip_space_and_one_newline(text, pos);
            if spaced > pos {
                if let Some((parsed_title, end)) = parse_title(text, spaced) {
                    title = parsed_title;
                    pos = skip_space_and_one_newline(text, end);
                } else {
                    pos = spaced;
                }
            }
        }
        (text.as_bytes().get(pos) == Some(&b')')).then_some((dest, title, pos + 1))
    }

    fn reference(&self, src_start: usize, close: usize) -> Option<(String, String, usize)> {
        let text = self.text;
        let after = close + 1;
        let lookup = |label: &str, end: usize| {
            self.refs
                .get(label)
                .map(|(dest, title)| (dest.clone(), title.clone(), end))
        };
        if let Some((label, end)) = parse_label(text, after) {
            return lookup(label, end);
        }
        let (inner, end) = parse_label(text, src_start - 1)?;
        if end != after {
            return None;
        }
        if text[after..].starts_with("[]") {
            return lookup(inner, after + 2);
        }
        lookup(inner, after)
    }
}

fn find_backtick_close(text: &str, from: usize, length: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut index = from;
    while index < bytes.len() {
        if bytes[index] != b'`' {
            index += 1;
            continue;
        }
        let start = index;
        while index < bytes.len() && bytes[index] == b'`' {
            index += 1;
        }
        if index - start == length {
            return Some(start);
        }
    }
    None
}

fn delim_text(delim: &Delim) -> Inline {
    let mut text = String::new();
    for _ in 0..delim.count {
        text.push(char::from(delim.ch));
    }
    Inline::Text(text)
}

fn push_inline(out: &mut Vec<Inline>, node: Inline) {
    if let Inline::Text(piece) = &node
        && let Some(Inline::Text(existing)) = out.last_mut()
    {
        existing.push_str(piece);
        return;
    }
    out.push(node);
}

fn flatten(toks: Vec<Tok>) -> Vec<Inline> {
    let mut out = Vec::new();
    for tok in toks {
        match tok {
            Tok::Node(node) => push_inline(&mut out, node),
            Tok::Delim(delim) => push_inline(&mut out, delim_text(&delim)),
            Tok::Bracket(bracket) => push_inline(
                &mut out,
                Inline::Text(String::from(if bracket.image { "![" } else { "[" })),
            ),
        }
    }
    out
}

fn kind_index(ch: u8) -> usize {
    match ch {
        b'*' => 0,
        b'_' => 1,
        _ => 2,
    }
}

fn can_match(opener: &Delim, closer: &Delim) -> bool {
    if opener.ch != closer.ch || !opener.can_open {
        return false;
    }
    if opener.ch == b'~' {
        return opener.count == closer.count;
    }
    let odd_rule = (opener.can_close || closer.can_open)
        && (opener.orig + closer.orig).is_multiple_of(3)
        && !(opener.orig.is_multiple_of(3) && closer.orig.is_multiple_of(3));
    !odd_rule
}

fn build(toks: Vec<Tok>) -> Vec<Inline> {
    let mut out: Vec<Tok> = Vec::with_capacity(toks.len());
    let mut openers: Vec<usize> = Vec::new();
    let mut bottoms = [[[0usize; 3]; 2]; 3];
    for tok in toks {
        let Tok::Delim(mut closer) = tok else {
            out.push(tok);
            continue;
        };
        while closer.can_close && closer.count > 0 {
            let key = (
                kind_index(closer.ch),
                usize::from(closer.can_open),
                closer.orig % 3,
            );
            let limit = bottoms[key.0][key.1][key.2];
            let found = (limit..openers.len()).rev().find(|&slot| {
                matches!(&out[openers[slot]], Tok::Delim(opener) if can_match(opener, &closer))
            });
            let Some(slot) = found else {
                bottoms[key.0][key.1][key.2] = openers.len();
                break;
            };
            let opener_at = openers[slot];
            let (ch, use_len) = match &out[opener_at] {
                Tok::Delim(opener) => {
                    let use_len = if opener.ch == b'~' {
                        opener.count
                    } else if opener.count >= 2 && closer.count >= 2 {
                        2
                    } else {
                        1
                    };
                    (opener.ch, use_len)
                }
                _ => break,
            };
            let content = flatten(out.split_off(opener_at + 1));
            let node = match (ch, use_len) {
                (b'~', _) => Inline::Strikethrough(content),
                (_, 2) => Inline::Strong(content),
                _ => Inline::Emphasis(content),
            };
            let opener_empty = match &mut out[opener_at] {
                Tok::Delim(opener) => {
                    opener.count -= use_len;
                    opener.count == 0
                }
                _ => false,
            };
            closer.count -= use_len;
            if opener_empty {
                out.truncate(opener_at);
                openers.truncate(slot);
            } else {
                openers.truncate(slot + 1);
            }
            out.push(Tok::Node(node));
            for row in &mut bottoms {
                for cell in row {
                    for value in cell {
                        *value = (*value).min(openers.len());
                    }
                }
            }
        }
        if closer.count == 0 {
            continue;
        }
        if closer.can_open {
            openers.push(out.len());
            out.push(Tok::Delim(closer));
        } else {
            out.push(Tok::Node(delim_text(&closer)));
        }
    }
    flatten(out)
}
