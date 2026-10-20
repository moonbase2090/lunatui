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

#[test]
fn live_dashboard_bench_compares_both_runtimes_at_the_requested_size()
-> Result<(), Box<dyn std::error::Error>> {
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
            "--bench",
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
        "live_dashboard comparison failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lunatui = stdout
        .lines()
        .find(|line| line.contains("lunatui live_dashboard"))
        .ok_or("lunatui benchmark summary missing")?;
    let ratatui = stdout
        .lines()
        .find(|line| line.contains("ratatui 0.30.2 live_dashboard"))
        .ok_or("ratatui benchmark summary missing")?;
    for line in [lunatui, ratatui] {
        assert!(line.contains("80x24, 4 frames"), "{line}");
        assert!(line.contains("p50 "), "{line}");
        assert!(line.contains("p99 "), "{line}");
    }
    Ok(())
}
