// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

#[test]
fn demo_snapshot_prints_hello_line() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO"))
        .current_dir(workspace_root())
        .args([
            "run",
            "--quiet",
            "-p",
            "lunatui",
            "--features",
            "app",
            "--example",
            "demo",
            "--",
            "--snapshot",
            "--width",
            "80",
            "--height",
            "24",
        ])
        .output()?;
    assert!(
        output.status.success(),
        "demo snapshot failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello, lunatui"), "{stdout}");
    assert!(stdout.contains("PR 23"), "{stdout}");
    Ok(())
}
