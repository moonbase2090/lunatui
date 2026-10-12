// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("../README.md")]

use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn live_dashboard_bench_prints_percentiles() {
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
        .output()
        .expect("spawn live_dashboard");
    assert!(
        output.status.success(),
        "live_dashboard failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("p50"));
    assert!(stdout.contains("ratatui_dashboard"));
}

#[test]
fn unicode_stress_renders_to_stdout() {
    let cargo = env!("CARGO");
    let output = Command::new(cargo)
        .current_dir(workspace_root())
        .args([
            "run",
            "--quiet",
            "-p",
            "lunatui",
            "--example",
            "unicode_stress",
        ])
        .output()
        .expect("spawn unicode_stress");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("unicode stress"));
    assert!(stdout.contains("🇺🇸"));
}
