// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::fs;
use std::io;
use std::process::ExitCode;

use crate::git;

const CODEOWNERS: &str = ".github/CODEOWNERS";
const SIGNOFF_LABEL: &str = "mb2090-signoff";

pub(crate) fn run(base: &str, head: &str, labeled: bool) -> ExitCode {
    match protected_changes(base, head) {
        Ok(protected) if protected.is_empty() || labeled => {
            println!("xtask signoff: ok");
            ExitCode::SUCCESS
        }
        Ok(protected) => {
            for path in &protected {
                eprintln!("{path}: lint, CI or dependency change");
            }
            eprintln!(
                "xtask signoff: these changes need MB2090 sign-off (CODEOWNERS review and the `{SIGNOFF_LABEL}` label)"
            );
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("xtask signoff: {error}");
            ExitCode::FAILURE
        }
    }
}

fn protected_changes(base: &str, head: &str) -> io::Result<Vec<String>> {
    git::enter_toplevel()?;
    let codeowners = fs::read_to_string(CODEOWNERS)?;
    let changed = git::changed_files(base, head)?;
    Ok(needs_signoff(&changed, &codeowners)
        .into_iter()
        .map(str::to_owned)
        .collect())
}

fn needs_signoff<'a>(changed: &'a [String], codeowners: &str) -> Vec<&'a str> {
    let patterns: Vec<&str> = codeowners
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split_whitespace().next())
        .collect();
    changed
        .iter()
        .map(String::as_str)
        .filter(|path| patterns.iter().any(|pattern| is_protected(path, pattern)))
        .collect()
}

fn is_protected(path: &str, pattern: &str) -> bool {
    let anchored = pattern.trim_start_matches('/');
    match anchored.strip_suffix('/') {
        Some(directory) => path
            .strip_prefix(directory)
            .is_some_and(|rest| rest.starts_with('/')),
        None => path == anchored,
    }
}

#[cfg(test)]
mod tests {
    use super::needs_signoff;

    const CODEOWNERS: &str = "# Lint, CI and dependency configuration needs MB2090 sign-off.

/Cargo.toml @mb2090
/Cargo.lock @mb2090
/.github/ @mb2090
";

    fn protected(changed: &[&str]) -> Vec<String> {
        let changed: Vec<String> = changed.iter().map(|path| (*path).to_owned()).collect();
        needs_signoff(&changed, CODEOWNERS)
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn a_file_pattern_protects_exactly_that_root_file() {
        assert_eq!(
            protected(&["Cargo.toml", "lunatui/Cargo.toml", "Cargo.lock"]),
            ["Cargo.toml", "Cargo.lock"]
        );
    }

    #[test]
    fn a_directory_pattern_protects_everything_under_it() {
        assert_eq!(
            protected(&[
                ".github/workflows/ci.yml",
                ".github/CODEOWNERS",
                "docs/lints.md"
            ]),
            [".github/workflows/ci.yml", ".github/CODEOWNERS"]
        );
    }

    #[test]
    fn feature_work_needs_no_signoff() {
        assert_eq!(
            protected(&["benches/src/report.rs", "examples/hello_box.rs"]),
            Vec::<String>::new()
        );
    }
}
