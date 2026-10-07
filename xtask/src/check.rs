// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod comments;
mod docs;
mod expectations;
mod manifests;

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::git;

pub(crate) use comments::LICENSE_HEADER;

const LINTS_DOC: &str = "docs/lints.md";

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Violation {
    pub(crate) path: PathBuf,
    pub(crate) line: usize,
    pub(crate) message: String,
}

impl Violation {
    pub(crate) fn new(path: &Path, line: usize, message: impl Into<String>) -> Self {
        Self {
            path: path.to_owned(),
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.path.display(), self.line, self.message)
    }
}

pub(crate) fn run() -> ExitCode {
    match git::enter_toplevel().and_then(|()| violations()) {
        Ok(found) if found.is_empty() => {
            println!("xtask check: ok");
            ExitCode::SUCCESS
        }
        Ok(found) => {
            for violation in &found {
                eprintln!("{violation}");
            }
            eprintln!("xtask check: {} violation(s)", found.len());
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("xtask check: {error}");
            ExitCode::FAILURE
        }
    }
}

fn violations() -> io::Result<Vec<Violation>> {
    let mut found = Vec::new();
    let mut expected = Vec::new();
    for path in git::work_tree_files(Path::new("."), "*.rs")? {
        let source = fs::read_to_string(&path)?;
        found.extend(comments::violations(&path, &source));
        let file = syn::parse_file(&source)
            .map_err(|error| io::Error::other(format!("{}: {error}", path.display())))?;
        found.extend(docs::violations(&path, &file));
        expected.extend(expectations::in_file(&path, &file));
    }
    let lints_doc = fs::read_to_string(LINTS_DOC)?;
    found.extend(expectations::violations(
        &expected,
        &expectations::listed(&lints_doc),
        Path::new(LINTS_DOC),
    ));
    for path in git::work_tree_files(Path::new("."), "*Cargo.toml")? {
        if path != Path::new("Cargo.toml") {
            found.extend(manifests::violations(&path, &fs::read_to_string(&path)?));
        }
    }
    found.sort();
    Ok(found)
}
