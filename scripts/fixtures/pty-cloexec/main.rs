// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::fs::File;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use lunatui_term::{PtyProcess, PtySize};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ready = PathBuf::from(std::env::var_os("PTY_PROBE_READY").ok_or("missing ready path")?);
    let release =
        PathBuf::from(std::env::var_os("PTY_PROBE_RELEASE").ok_or("missing release path")?);
    let (opened_tx, opened_rx) = mpsc::channel();
    let (stop_tx, stop_rx) = mpsc::channel();
    let worker = thread::spawn(move || -> std::io::Result<()> {
        let mut command = Command::new("/bin/sleep");
        command.arg("30");
        let process = PtyProcess::spawn(command, PtySize::new(80, 24)?)?;
        let _ = opened_tx.send(());
        let _ = stop_rx.recv_timeout(Duration::from_secs(15));
        drop(process);
        Ok(())
    });
    let inspected = (|| -> Result<(bool, String), Box<dyn std::error::Error>> {
        let deadline = Instant::now() + Duration::from_secs(10);
        let paused = loop {
            if ready.exists() {
                break true;
            }
            match opened_rx.try_recv() {
                Ok(()) => break false,
                Err(mpsc::TryRecvError::Disconnected) => return Err("PTY allocation failed".into()),
                Err(mpsc::TryRecvError::Empty) => {}
            }
            if Instant::now() >= deadline {
                return Err("PTY allocation timed out".into());
            }
            thread::sleep(Duration::from_millis(1));
        };
        let output_path = ready.with_extension("fds");
        let status = Command::new("/bin/sh")
            .args([
                "-c",
                "for fd in /proc/$$/fd/*; do readlink \"$fd\" || true; done",
            ])
            .stdin(Stdio::null())
            .stdout(File::create(&output_path)?)
            .stderr(Stdio::null())
            .status()?;
        if !status.success() {
            return Err("descriptor inspection failed".into());
        }
        Ok((paused, std::fs::read_to_string(output_path)?))
    })();
    let released = std::fs::write(&release, b"release");
    let _ = stop_tx.send(());
    let joined = worker.join().map_err(|_| "PTY worker panicked");
    released?;
    joined??;
    let (paused, output) = inspected?;
    let leaked = output.lines().any(|line| {
        line == "/dev/ptmx" || line == "/dev/pts/ptmx" || line.starts_with("/dev/pts/")
    });
    println!("paused_openpty={paused} inherited_pty={leaked}");
    print!("{output}");
    if leaked {
        std::process::exit(1);
    }
    Ok(())
}
