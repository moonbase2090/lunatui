// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

#[test]
fn live_dashboard_bench_prints_percentiles() -> Result<(), Box<dyn std::error::Error>> {
    let cargo = env!("CARGO");
    let output = Command::new(cargo)
        .current_dir(workspace_root())
        .args([
            "run",
            "--quiet",
            "-p",
            "lunatui",
            "--example",
            "live_dashboard",
            "--",
            "--width",
            "80",
            "--height",
            "24",
            "--frames",
            "4",
        ])
        .output()?;
    assert!(
        output.status.success(),
        "live_dashboard failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("p50"));
    assert!(stdout.contains("p99"));
    assert!(stdout.contains("max"));
    assert!(stdout.contains("8.000 ms target"));
    assert!(stdout.contains("ratatui_dashboard"));
    Ok(())
}
