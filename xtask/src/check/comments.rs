// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

mod scanner;

use std::path::Path;

use scanner::{Comment, CommentKind, scan};

use super::Violation;

pub(crate) const LICENSE_HEADER: &str = "\
// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT
";

const SAFETY_PREFIX: &str = "// SAFETY: ";

const HEADER_MESSAGE: &str = "missing or altered license header (M14): the file must start with the approved MIT SPDX header and a blank line";

const LINE_COMMENT_MESSAGE: &str = "comment not allowed (M14): the only approved comments are the license header and `// SAFETY:` directly before `unsafe`";

const INNER_DOC_MESSAGE: &str = "`//!` inner docs are not allowed (M14): crate docs use `#![doc = include_str!(\"README.md\")]`";

const BLOCK_COMMENT_MESSAGE: &str = "block comments are not allowed (M14)";

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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{LICENSE_HEADER, violations};

    fn violation_lines(body: &str) -> Vec<usize> {
        let source = format!("{LICENSE_HEADER}\n{body}");
        violations(Path::new("a.rs"), &source)
            .iter()
            .map(|violation| violation.line)
            .collect()
    }

    #[test]
    fn a_file_without_the_license_header_is_rejected() {
        let found = violations(Path::new("a.rs"), "fn main() {}\n");
        assert_eq!(found.iter().map(|v| v.line).collect::<Vec<_>>(), [1]);
    }

    #[test]
    fn an_altered_license_header_is_rejected() {
        let source = LICENSE_HEADER.replace("MIT", "Apache-2.0") + "\nfn main() {}\n";
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
        assert_eq!(violation_lines("// note\nfn main() {}\n"), [4]);
    }

    #[test]
    fn inner_and_block_doc_comments_are_rejected() {
        assert_eq!(
            violation_lines("//! crate\n/** item */\nfn main() {}\n"),
            [4, 5]
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
        assert_eq!(violation_lines("// SAFETY: nothing here\nfn f() {}\n"), [4]);
    }

    #[test]
    fn a_plain_comment_before_unsafe_is_rejected() {
        assert_eq!(violation_lines("// trust me\nunsafe fn f() {}\n"), [4]);
    }
}
