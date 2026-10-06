// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CommentKind {
    Line,
    OuterDoc,
    InnerDoc,
    Block,
    OuterBlockDoc,
    InnerBlockDoc,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Comment<'a> {
    pub(crate) line: usize,
    pub(crate) kind: CommentKind,
    pub(crate) text: &'a str,
    pub(crate) precedes_unsafe: bool,
}

pub(crate) fn scan(source: &str) -> Vec<Comment<'_>> {
    let mut scanner = Scanner {
        source,
        pos: 0,
        line: 1,
        comments: Vec::new(),
    };
    scanner.run();
    scanner.comments
}

fn line_kind(text: &str) -> CommentKind {
    if text.starts_with("////") {
        CommentKind::Line
    } else if text.starts_with("///") {
        CommentKind::OuterDoc
    } else if text.starts_with("//!") {
        CommentKind::InnerDoc
    } else {
        CommentKind::Line
    }
}

fn block_kind(text: &str) -> CommentKind {
    if text.starts_with("/**/") || text.starts_with("/***") {
        CommentKind::Block
    } else if text.starts_with("/**") {
        CommentKind::OuterBlockDoc
    } else if text.starts_with("/*!") {
        CommentKind::InnerBlockDoc
    } else {
        CommentKind::Block
    }
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_' || !byte.is_ascii()
}

fn is_ident_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || !byte.is_ascii()
}

fn next_token_is_unsafe(rest: &str) -> bool {
    rest.trim_start()
        .strip_prefix("unsafe")
        .is_some_and(|after| !after.bytes().next().is_some_and(is_ident_continue))
}

struct Scanner<'a> {
    source: &'a str,
    pos: usize,
    line: usize,
    comments: Vec<Comment<'a>>,
}

impl Scanner<'_> {
    fn run(&mut self) {
        while let Some(byte) = self.peek(0) {
            match (byte, self.peek(1)) {
                (b'/', Some(b'/')) => self.line_comment(),
                (b'/', Some(b'*')) => self.block_comment(),
                (b'"', _) => self.quoted(),
                (b'\'', _) => self.char_or_lifetime(),
                (byte, _) if is_ident_start(byte) => self.identifier(),
                _ => self.advance(1),
            }
        }
    }

    fn peek(&self, offset: usize) -> Option<u8> {
        self.source.as_bytes().get(self.pos + offset).copied()
    }

    fn advance(&mut self, count: usize) {
        let end = (self.pos + count).min(self.source.len());
        for &byte in &self.source.as_bytes()[self.pos..end] {
            if byte == b'\n' {
                self.line += 1;
            }
        }
        self.pos = end;
    }

    fn advance_past(&mut self, needle: &[u8]) {
        let rest = &self.source.as_bytes()[self.pos..];
        let skip = rest
            .windows(needle.len())
            .position(|window| window == needle)
            .map_or(rest.len(), |found| found + needle.len());
        self.advance(skip);
    }

    fn push(&mut self, start: usize, line: usize, kind: CommentKind) {
        let text = &self.source[start..self.pos];
        let precedes_unsafe = next_token_is_unsafe(&self.source[self.pos..]);
        self.comments.push(Comment {
            line,
            kind,
            text,
            precedes_unsafe,
        });
    }

    fn line_comment(&mut self) {
        let (start, line) = (self.pos, self.line);
        let length = self.source.as_bytes()[start..]
            .iter()
            .position(|&byte| byte == b'\n')
            .unwrap_or(self.source.len() - start);
        self.advance(length);
        self.push(start, line, line_kind(&self.source[start..self.pos]));
    }

    fn block_comment(&mut self) {
        let (start, line) = (self.pos, self.line);
        self.advance(2);
        let mut depth = 1;
        while depth > 0 {
            match (self.peek(0), self.peek(1)) {
                (None, _) => break,
                (Some(b'/'), Some(b'*')) => {
                    depth += 1;
                    self.advance(2);
                }
                (Some(b'*'), Some(b'/')) => {
                    depth -= 1;
                    self.advance(2);
                }
                _ => self.advance(1),
            }
        }
        self.push(start, line, block_kind(&self.source[start..self.pos]));
    }

    fn quoted(&mut self) {
        self.advance(1);
        while let Some(byte) = self.peek(0) {
            match byte {
                b'\\' => self.advance(2),
                b'"' => {
                    self.advance(1);
                    return;
                }
                _ => self.advance(1),
            }
        }
    }

    fn char_or_lifetime(&mut self) {
        let rest = &self.source[self.pos + 1..];
        let mut chars = rest.chars();
        match (chars.next(), chars.next()) {
            (Some('\\'), _) => {
                self.advance(3);
                self.advance_past(b"'");
            }
            (Some(c), Some('\'')) => self.advance(1 + c.len_utf8() + 1),
            _ => self.advance(1),
        }
    }

    fn identifier(&mut self) {
        let start = self.pos;
        while self.peek(0).is_some_and(is_ident_continue) {
            self.advance(1);
        }
        match (&self.source[start..self.pos], self.peek(0)) {
            ("r" | "br" | "cr", Some(b'"' | b'#')) => self.raw_string(),
            ("b" | "c", Some(b'"')) => self.quoted(),
            ("b", Some(b'\'')) => self.char_or_lifetime(),
            _ => {}
        }
    }

    fn raw_string(&mut self) {
        let hashes = self.source.as_bytes()[self.pos..]
            .iter()
            .take_while(|&&byte| byte == b'#')
            .count();
        if self.peek(hashes) == Some(b'"') {
            self.advance(hashes + 1);
            let mut closing = vec![b'"'];
            closing.resize(hashes + 1, b'#');
            self.advance_past(&closing);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Comment, CommentKind, scan};

    fn kinds(source: &str) -> Vec<(usize, CommentKind)> {
        scan(source)
            .iter()
            .map(|comment| (comment.line, comment.kind))
            .collect()
    }

    #[test]
    fn finds_line_and_block_comments() {
        assert_eq!(
            kinds("let a = 1; // note\n/* block */ let b = 2;\n"),
            [(1, CommentKind::Line), (2, CommentKind::Block)]
        );
    }

    #[test]
    fn keeps_the_comment_text() {
        let comments = scan("let a = 1; // note\n");
        let expected = Comment {
            line: 1,
            kind: CommentKind::Line,
            text: "// note",
            precedes_unsafe: false,
        };
        assert_eq!(comments, [expected]);
    }

    #[test]
    fn ignores_comment_markers_inside_strings() {
        assert_eq!(
            kinds("let url = \"https://example.com\"; let s = \"/* \\\" */\";\n"),
            []
        );
    }

    #[test]
    fn ignores_comment_markers_inside_raw_strings() {
        assert_eq!(
            kinds("let s = r#\"a \"// b\" c\"#; let t = br\"/*\";\n"),
            []
        );
    }

    #[test]
    fn raw_identifiers_are_not_strings() {
        assert_eq!(
            kinds("let r#type = 1; // after\n"),
            [(1, CommentKind::Line)]
        );
    }

    #[test]
    fn char_literals_and_lifetimes_are_not_strings() {
        let source =
            "fn f<'a>(x: &'a str) -> char { '/' } // c\nlet q = '\"'; let e = '\\''; // d\n";
        assert_eq!(
            kinds(source),
            [(1, CommentKind::Line), (2, CommentKind::Line)]
        );
    }

    #[test]
    fn byte_strings_and_byte_chars_are_not_comments() {
        assert_eq!(
            kinds("let a = b\"//\"; let c = b'/'; // e\n"),
            [(1, CommentKind::Line)]
        );
    }

    #[test]
    fn nested_block_comments_end_at_the_outer_close() {
        assert_eq!(
            kinds("/* a /* b */ c */ let x = 1; // tail\n"),
            [(1, CommentKind::Block), (1, CommentKind::Line)]
        );
    }

    #[test]
    fn classifies_doc_comments_like_rustc() {
        let source =
            "/// outer\n//! inner\n//// plain\n/** outer */\n/*! inner */\n/**/\n/*** plain */\n";
        assert_eq!(
            kinds(source),
            [
                (1, CommentKind::OuterDoc),
                (2, CommentKind::InnerDoc),
                (3, CommentKind::Line),
                (4, CommentKind::OuterBlockDoc),
                (5, CommentKind::InnerBlockDoc),
                (6, CommentKind::Block),
                (7, CommentKind::Block),
            ]
        );
    }

    #[test]
    fn counts_lines_inside_strings_and_block_comments() {
        assert_eq!(
            kinds("let s = \"a\nb\";\n/* x\ny */\n// here\n"),
            [(3, CommentKind::Block), (5, CommentKind::Line)]
        );
    }

    #[test]
    fn marks_a_comment_directly_before_unsafe() {
        let comments = scan("// SAFETY: ok\nunsafe { f() }\n// x\nlet y = unsafe_fn();\n");
        let precedes: Vec<bool> = comments.iter().map(|c| c.precedes_unsafe).collect();
        assert_eq!(precedes, [true, false]);
    }
}
