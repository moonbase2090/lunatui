// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::path::Path;

use super::Violation;

pub(crate) const LICENSE_HEADER: &str = "\
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
";

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

const SAFETY_PREFIX: &str = "// SAFETY: ";

const HEADER_MESSAGE: &str = "missing or altered license header (M14): the file must start with the approved MPL-2.0 header and a blank line";

const LINE_COMMENT_MESSAGE: &str = "comment not allowed (M14): the only approved comments are the license header and `// SAFETY:` directly before `unsafe`";

const INNER_DOC_MESSAGE: &str = "`//!` inner docs are not allowed (M14): crate docs use `#![doc = include_str!(\"README.md\")]`";

const BLOCK_COMMENT_MESSAGE: &str = "block comments are not allowed (M14)";

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

pub(crate) fn violations(path: &Path, source: &str) -> Vec<Violation> {
    let Some(body) = source.strip_prefix(LICENSE_HEADER) else {
        return vec![Violation::new(path, 1, HEADER_MESSAGE)];
    };
    if !body.starts_with('\n') {
        return vec![Violation::new(path, 1, HEADER_MESSAGE)];
    }
    let header_lines = LICENSE_HEADER.lines().count();
    let lines: Vec<&str> = source.lines().collect();
    let comments: Vec<Comment<'_>> = scan(source)
        .into_iter()
        .filter(|comment| comment.line > header_lines)
        .collect();

    let mut found = Vec::new();
    let mut index = 0;
    while let Some(comment) = comments.get(index) {
        match comment.kind {
            CommentKind::OuterDoc => index += 1,
            CommentKind::Line => {
                let run = line_run(&comments[index..], &lines);
                if !is_safety_run(run, &lines) {
                    found.extend(
                        run.iter()
                            .map(|c| Violation::new(path, c.line, LINE_COMMENT_MESSAGE)),
                    );
                }
                index += run.len();
            }
            CommentKind::InnerDoc => {
                found.push(Violation::new(path, comment.line, INNER_DOC_MESSAGE));
                index += 1;
            }
            CommentKind::Block | CommentKind::OuterBlockDoc | CommentKind::InnerBlockDoc => {
                found.push(Violation::new(path, comment.line, BLOCK_COMMENT_MESSAGE));
                index += 1;
            }
        }
    }
    found
}

fn line_run<'c, 'a>(comments: &'c [Comment<'a>], lines: &[&str]) -> &'c [Comment<'a>] {
    let continues = comments
        .windows(2)
        .take_while(|pair| {
            pair[1].kind == CommentKind::Line
                && pair[1].line == pair[0].line + 1
                && starts_its_line(&pair[1], lines)
        })
        .count();
    &comments[..=continues]
}

fn is_safety_run(run: &[Comment<'_>], lines: &[&str]) -> bool {
    let (Some(first), Some(last)) = (run.first(), run.last()) else {
        return false;
    };
    first.text.starts_with(SAFETY_PREFIX) && starts_its_line(first, lines) && last.precedes_unsafe
}

fn starts_its_line(comment: &Comment<'_>, lines: &[&str]) -> bool {
    lines
        .get(comment.line - 1)
        .is_some_and(|line| line.trim_start().starts_with(comment.text))
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
    use std::path::Path;

    use super::{Comment, CommentKind, LICENSE_HEADER, scan, violations};

    fn kinds(source: &str) -> Vec<(usize, CommentKind)> {
        scan(source)
            .iter()
            .map(|comment| (comment.line, comment.kind))
            .collect()
    }

    fn violation_lines(body: &str) -> Vec<usize> {
        let source = format!("{LICENSE_HEADER}\n{body}");
        violations(Path::new("a.rs"), &source)
            .iter()
            .map(|violation| violation.line)
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

    #[test]
    fn a_file_without_the_license_header_is_rejected() {
        let found = violations(Path::new("a.rs"), "fn main() {}\n");
        assert_eq!(found.iter().map(|v| v.line).collect::<Vec<_>>(), [1]);
    }

    #[test]
    fn an_altered_license_header_is_rejected() {
        let source = LICENSE_HEADER.replace("v. 2.0", "v. 2.1") + "\nfn main() {}\n";
        assert_eq!(violations(Path::new("a.rs"), &source).len(), 1);
    }

    #[test]
    fn the_license_header_and_code_alone_are_clean() {
        assert_eq!(violation_lines("fn main() {}\n"), []);
    }

    #[test]
    fn outer_doc_comments_are_allowed() {
        assert_eq!(violation_lines("/// Does a thing.\npub fn f() {}\n"), []);
    }

    #[test]
    fn plain_comments_are_rejected() {
        assert_eq!(violation_lines("// note\nfn main() {}\n"), [5]);
    }

    #[test]
    fn inner_and_block_doc_comments_are_rejected() {
        assert_eq!(
            violation_lines("//! crate\n/** item */\nfn main() {}\n"),
            [5, 6]
        );
    }

    #[test]
    fn a_safety_comment_run_before_unsafe_is_allowed() {
        let body = "fn f() {\n    // SAFETY: the pointer is valid\n    // for the whole call.\n    unsafe { g() }\n}\n";
        assert_eq!(violation_lines(body), []);
    }

    #[test]
    fn a_safety_comment_before_unsafe_impl_is_allowed() {
        assert_eq!(
            violation_lines("// SAFETY: forwards to System.\nunsafe impl Send for T {}\n"),
            []
        );
    }

    #[test]
    fn a_safety_comment_not_before_unsafe_is_rejected() {
        assert_eq!(violation_lines("// SAFETY: nothing here\nfn f() {}\n"), [5]);
    }

    #[test]
    fn a_plain_comment_before_unsafe_is_rejected() {
        assert_eq!(violation_lines("// trust me\nunsafe fn f() {}\n"), [5]);
    }
}
