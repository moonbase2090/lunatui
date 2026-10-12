// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

#[test]
fn live_dashboard_snapshot_prints_header_and_log() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO"))
        .current_dir(workspace_root())
        .args([
            "run",
            "--quiet",
            "-p",
            "lunatui",
            "--example",
            "live_dashboard",
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
        "live_dashboard snapshot failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("lunatui live dashboard"), "{stdout}");
    assert!(stdout.contains("scrolling log"), "{stdout}");
    assert!(stdout.contains("processed batch"), "{stdout}");
    Ok(())
}
