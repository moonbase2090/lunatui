// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::Violation;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Expectation {
    pub(crate) path: PathBuf,
    pub(crate) line: usize,
    pub(crate) lint: String,
    pub(crate) has_reason: bool,
}

pub(crate) type Listed = BTreeMap<(PathBuf, String), usize>;

pub(crate) fn in_file(_path: &Path, _file: &syn::File) -> Vec<Expectation> {
    Vec::new()
}

pub(crate) fn listed(_lints_doc: &str) -> Listed {
    Listed::new()
}

pub(crate) fn violations(
    _found: &[Expectation],
    _listed: &Listed,
    _lints_doc: &Path,
) -> Vec<Violation> {
    Vec::new()
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
