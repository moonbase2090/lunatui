// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Attribute, Meta, Token};

use super::Violation;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Expectation {
    pub(crate) path: PathBuf,
    pub(crate) line: usize,
    pub(crate) lint: String,
    pub(crate) has_reason: bool,
}

pub(crate) type Listed = BTreeMap<(PathBuf, String), usize>;

const EXPECTATIONS_HEADING: &str = "## Lint expectations";

pub(crate) fn in_file(path: &Path, file: &syn::File) -> Vec<Expectation> {
    let mut collector = Collector {
        path,
        found: Vec::new(),
    };
    collector.visit_file(file);
    collector.found
}

pub(crate) fn listed(lints_doc: &str) -> Listed {
    let mut listed = Listed::new();
    let mut in_expectations = false;
    for (index, line) in lints_doc.lines().enumerate() {
        if line.starts_with("## ") {
            in_expectations = line.starts_with(EXPECTATIONS_HEADING);
            continue;
        }
        let mut cells = line.split('|').skip(1);
        if let (true, Some(location), Some(lint)) = (
            in_expectations,
            cells.next().and_then(first_code_span),
            cells.next().and_then(first_code_span),
        ) {
            listed.insert((PathBuf::from(location), lint.to_owned()), index + 1);
        }
    }
    listed
}

pub(crate) fn violations(
    found: &[Expectation],
    listed: &Listed,
    lints_doc: &Path,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let mut used = BTreeSet::new();
    for expectation in found {
        if !expectation.has_reason {
            violations.push(Violation::new(
                &expectation.path,
                expectation.line,
                "`#[expect]` needs `reason = \"...\"` (M16)",
            ));
        }
        let key = (expectation.path.clone(), expectation.lint.clone());
        if listed.contains_key(&key) {
            used.insert(key);
        } else {
            violations.push(Violation::new(
                &expectation.path,
                expectation.line,
                format!(
                    "`#[expect({})]` is not listed in docs/lints.md (M16)",
                    expectation.lint
                ),
            ));
        }
    }
    for (key, line) in listed {
        if !used.contains(key) {
            violations.push(Violation::new(
                lints_doc,
                *line,
                format!(
                    "listed expectation `{}` in `{}` no longer exists",
                    key.1,
                    key.0.display()
                ),
            ));
        }
    }
    violations
}

fn first_code_span(cell: &str) -> Option<&str> {
    let start = cell.find('`')? + 1;
    let length = cell[start..].find('`')?;
    Some(&cell[start..start + length])
}

fn lint_name(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

struct Collector<'p> {
    path: &'p Path,
    found: Vec<Expectation>,
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        if attr.path().is_ident("expect") {
            let line = attr.span().start().line;
            match attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated) {
                Ok(metas) => {
                    let has_reason = metas.iter().any(
                        |meta| matches!(meta, Meta::NameValue(pair) if pair.path.is_ident("reason")),
                    );
                    for meta in &metas {
                        if let Meta::Path(lint) = meta {
                            self.found.push(Expectation {
                                path: self.path.to_owned(),
                                line,
                                lint: lint_name(lint),
                                has_reason,
                            });
                        }
                    }
                }
                Err(_) => self.found.push(Expectation {
                    path: self.path.to_owned(),
                    line,
                    lint: "<unparsable #[expect]>".to_owned(),
                    has_reason: false,
                }),
            }
        }
        visit::visit_attribute(self, attr);
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{Expectation, Listed, in_file, listed, violations};

    const LINTS_DOC: &str = "\
# Lints

## Lint rules (M15)

| Rule | Enforced by | Mistake it prevents | Added in |
|---|---|---|---|
| `not-an-expectation.rs` | `x` | y | z |

## Lint expectations (M16)

| Location | Lint | Reason | Added in |
|---|---|---|---|
| `benches/src/calibration.rs`, `grow_buffer` | `clippy::same_item_push` | Workload | PR 02 |

## Other
";

    fn summary(source: &str) -> syn::Result<Vec<(usize, String, bool)>> {
        let file = syn::parse_file(source)?;
        Ok(in_file(Path::new("a.rs"), &file)
            .into_iter()
            .map(|e| (e.line, e.lint, e.has_reason))
            .collect())
    }

    fn expectation(path: &str, line: usize, lint_name: &str, has_reason: bool) -> Expectation {
        Expectation {
            path: PathBuf::from(path),
            line,
            lint: lint_name.to_owned(),
            has_reason,
        }
    }

    #[test]
    fn finds_inner_and_outer_expectations() -> syn::Result<()> {
        let source = "#![expect(missing_docs, reason = \"r\")]\n#[expect(clippy::same_item_push, reason = \"r\")]\nfn f() {}\n";
        assert_eq!(
            summary(source)?,
            [
                (1, "missing_docs".to_owned(), true),
                (2, "clippy::same_item_push".to_owned(), true),
            ]
        );
        Ok(())
    }

    #[test]
    fn finds_expectations_inside_function_bodies() -> syn::Result<()> {
        let source =
            "fn f() {\n    #[expect(unused_variables, reason = \"r\")]\n    let x = 1;\n}\n";
        assert_eq!(summary(source)?, [(2, "unused_variables".to_owned(), true)]);
        Ok(())
    }

    #[test]
    fn records_each_lint_of_a_multi_lint_expectation() -> syn::Result<()> {
        let source = "#[expect(dead_code, unused, reason = \"r\")]\nfn f() {}\n";
        assert_eq!(
            summary(source)?,
            [
                (1, "dead_code".to_owned(), true),
                (1, "unused".to_owned(), true),
            ]
        );
        Ok(())
    }

    #[test]
    fn notices_a_missing_reason() -> syn::Result<()> {
        assert_eq!(
            summary("#[expect(dead_code)]\nfn f() {}\n")?,
            [(1, "dead_code".to_owned(), false)]
        );
        Ok(())
    }

    #[test]
    fn reads_only_the_expectations_table() {
        let expected = Listed::from([(
            (
                PathBuf::from("benches/src/calibration.rs"),
                "clippy::same_item_push".to_owned(),
            ),
            13,
        )]);
        assert_eq!(listed(LINTS_DOC), expected);
    }

    #[test]
    fn a_listed_expectation_with_a_reason_is_clean() {
        let found = [expectation(
            "benches/src/calibration.rs",
            53,
            "clippy::same_item_push",
            true,
        )];
        assert_eq!(
            violations(&found, &listed(LINTS_DOC), Path::new("docs/lints.md")),
            []
        );
    }

    #[test]
    fn an_unlisted_expectation_is_reported_where_it_is() {
        let found = [
            expectation(
                "benches/src/calibration.rs",
                53,
                "clippy::same_item_push",
                true,
            ),
            expectation("xtask/src/main.rs", 9, "dead_code", true),
        ];
        let lines: Vec<(PathBuf, usize)> =
            violations(&found, &listed(LINTS_DOC), Path::new("docs/lints.md"))
                .into_iter()
                .map(|v| (v.path, v.line))
                .collect();
        assert_eq!(lines, [(PathBuf::from("xtask/src/main.rs"), 9)]);
    }

    #[test]
    fn a_stale_listing_is_reported_in_the_lints_doc() {
        let lines: Vec<(PathBuf, usize)> =
            violations(&[], &listed(LINTS_DOC), Path::new("docs/lints.md"))
                .into_iter()
                .map(|v| (v.path, v.line))
                .collect();
        assert_eq!(lines, [(PathBuf::from("docs/lints.md"), 13)]);
    }

    #[test]
    fn an_expectation_without_a_reason_is_reported() {
        let found = [expectation(
            "benches/src/calibration.rs",
            53,
            "clippy::same_item_push",
            false,
        )];
        assert_eq!(
            violations(&found, &listed(LINTS_DOC), Path::new("docs/lints.md")).len(),
            1
        );
    }
}
