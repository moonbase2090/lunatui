// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::env;
use std::io;
use std::path::PathBuf;
use std::process::Command;

pub(crate) fn output(args: &[&str]) -> io::Result<String> {
    let output = Command::new("git").args(args).output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "`git {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    String::from_utf8(output.stdout).map_err(io::Error::other)
}

pub(crate) fn enter_toplevel() -> io::Result<()> {
    let toplevel = output(&["rev-parse", "--show-toplevel"])?;
    env::set_current_dir(toplevel.trim())
}

pub(crate) fn tracked_files(pattern: &str) -> io::Result<Vec<PathBuf>> {
    Ok(output(&["ls-files", "--", pattern])?
        .lines()
        .map(PathBuf::from)
        .collect())
}

pub(crate) fn changed_files(base: &str, head: &str) -> io::Result<Vec<String>> {
    Ok(
        output(&["diff", "--name-only", &format!("{base}...{head}")])?
            .lines()
            .map(str::to_owned)
            .collect(),
    )
}
