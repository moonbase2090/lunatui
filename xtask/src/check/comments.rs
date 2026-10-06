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

pub(crate) fn scan(_source: &str) -> Vec<Comment<'_>> {
    Vec::new()
}

pub(crate) fn violations(_path: &Path, _source: &str) -> Vec<Violation> {
    Vec::new()
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
        assert!(!violations(Path::new("a.rs"), &source).is_empty());
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
