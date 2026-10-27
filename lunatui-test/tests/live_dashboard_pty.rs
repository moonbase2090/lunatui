// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]
#![cfg(unix)]

use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

#[test]
fn live_dashboard_renders_in_a_pty_screen() -> Result<(), Box<dyn std::error::Error>> {
    let pty_runner = r"
import os, pty, sys
pid, fd = pty.fork()
if pid == 0:
    os.execvp(sys.argv[1], sys.argv[1:])
while True:
    try:
        chunk = os.read(fd, 4096)
    except OSError:
        break
    if not chunk:
        break
    sys.stdout.buffer.write(chunk)
    sys.stdout.buffer.flush()
_, status = os.waitpid(pid, 0)
sys.exit(os.waitstatus_to_exitcode(status))
";
    let output = Command::new("python3")
        .current_dir(workspace_root())
        .args([
            "-c",
            pty_runner,
            env!("CARGO"),
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
            "2",
        ])
        .output()?;
    assert!(
        output.status.success(),
        "live_dashboard pty run failed: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let mut parser = vt100::Parser::new(24, 80, 0);
    let mut rendered = None;
    for byte in &output.stdout {
        parser.process(std::slice::from_ref(byte));
        if parser.screen().alternate_screen() {
            let screen = parser.screen().contents();
            if screen.contains("lunatui live dashboard")
                && screen.contains("scrolling log")
                && screen.contains("processed batch")
            {
                rendered = Some(screen);
                break;
            }
        }
    }
    let screen = rendered.ok_or("pty screen did not contain the dashboard and log")?;
    assert!(screen.contains("lunatui live dashboard"), "{screen}");
    assert!(screen.contains("scrolling log"), "{screen}");
    assert!(screen.contains("processed batch"), "{screen}");
    Ok(())
}
