// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]
#![cfg(unix)]

use std::io::{self, Read};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use lunatui_term::{PtyProcess, PtySize};

type Test = Result<(), Box<dyn std::error::Error>>;

fn until(process: &mut PtyProcess, wanted: &str) -> io::Result<String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut output = Vec::new();
    let mut buffer = [0; 4096];
    loop {
        process.flush_input()?;
        match process.read(&mut buffer) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    String::from_utf8_lossy(&output).into_owned(),
                ));
            }
            Ok(count) => output.extend_from_slice(&buffer[..count]),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(error),
        }
        let text = String::from_utf8_lossy(&output);
        if text.contains(wanted) {
            return Ok(text.into_owned());
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(io::ErrorKind::TimedOut, text.into_owned()));
        }
        thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn dimensions_reject_empty_or_excessive_allocations() {
    for (cols, rows) in [(0, 1), (1, 0), (4097, 1), (1, 4097), (4096, 4096)] {
        assert!(PtySize::new(cols, rows).is_err());
    }
    assert!(PtySize::new(80, 24).is_ok());
    assert!(PtySize::new(1024, 1024).is_ok());
}

#[test]
fn child_gets_a_controlling_terminal_exact_argv_input_resize_and_exit_status() -> Test {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "test -t 0 && test -t 1 && test -t 2 || exit 9; printf 'ARG:%s\\n' \"$1\"; stty size; printf READY; read answer; printf 'REPLY:%s\\n' \"$answer\"; stty size; exit 7", "fixture", "space ; $literal"]);
    let mut process = PtyProcess::spawn(command, PtySize::new(80, 24)?)?;
    let output = until(&mut process, "READY")?;
    assert!(output.contains("ARG:space ; $literal"), "{output:?}");
    assert!(output.contains("24 80"), "{output:?}");
    process.resize(PtySize::new(100, 30)?)?;
    process.queue_input(b"hello child\n")?;
    let output = until(&mut process, "30 100")?;
    assert!(output.contains("REPLY:hello child"), "{output:?}");
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = process.try_wait()? {
            break status;
        }
        assert!(Instant::now() < deadline, "child failed to exit");
        thread::sleep(Duration::from_millis(2));
    };
    assert_eq!(status.code(), Some(7));
    assert_eq!(process.try_wait()?, Some(status));
    Ok(())
}

#[test]
fn control_c_reaches_the_child_foreground_process_group() -> Test {
    let mut command = Command::new("/bin/sh");
    command.args([
        "-c",
        "trap 'printf INTERRUPTED; exit 0' INT; printf READY; while :; do sleep 1; done",
    ]);
    let mut process = PtyProcess::spawn(command, PtySize::new(80, 24)?)?;
    until(&mut process, "READY")?;
    process.queue_input(&[3])?;
    until(&mut process, "INTERRUPTED")?;
    Ok(())
}

#[test]
fn input_backpressure_rejects_the_whole_extra_chunk() -> Test {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "printf READY; sleep 30"]);
    let mut process = PtyProcess::spawn(command, PtySize::new(80, 24)?)?;
    until(&mut process, "READY")?;
    process.queue_input(&vec![b'x'; 65536])?;
    let error = process
        .queue_input(b"overflow")
        .err()
        .ok_or("queue should be full")?;
    assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
    assert_eq!(process.pending_input(), 65536);
    Ok(())
}

#[test]
fn dropping_a_pane_kills_and_reaps_its_direct_child() -> Test {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "printf READY; sleep 30"]);
    let mut process = PtyProcess::spawn(command, PtySize::new(80, 24)?)?;
    until(&mut process, "READY")?;
    let pid = i32::try_from(process.id())?;
    let started = Instant::now();
    drop(process);
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "closing a pane waited for the command to finish"
    );
    let mut status = 0;
    let result = {
        // SAFETY: checks the retained child PID without sending any signal.
        unsafe { libc::waitpid(pid, &raw mut status, libc::WNOHANG) }
    };
    assert_eq!(result, -1);
    assert_eq!(
        io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
    Ok(())
}

#[test]
fn a_missing_executable_returns_an_error() -> Test {
    let result = PtyProcess::spawn(
        Command::new("/nonexistent/lunatui-pty-fixture"),
        PtySize::new(80, 24)?,
    );
    assert!(result.is_err());
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn the_master_descriptor_is_not_inherited_across_exec() -> Test {
    let mut command = Command::new("/bin/sh");
    command.args([
        "-c",
        "for fd in /proc/$$/fd/*; do readlink \"$fd\" || true; done; printf END",
    ]);
    let mut process = PtyProcess::spawn(command, PtySize::new(80, 24)?)?;
    let output = until(&mut process, "END")?;
    assert!(
        !output.contains("ptmx"),
        "child inherited its PTY master: {output}"
    );
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn closing_a_pane_also_kills_ordinary_descendants_in_its_process_group() -> Test {
    let mut command = Command::new("/bin/sh");
    command.args([
        "-c",
        "trap '' HUP; sleep 30 & printf 'DESCENDANT:%s\\nREADY' \"$!\"; wait",
    ]);
    let mut process = PtyProcess::spawn(command, PtySize::new(80, 24)?)?;
    let output = until(&mut process, "READY")?;
    let pid = output
        .split("DESCENDANT:")
        .nth(1)
        .and_then(|text| text.lines().next())
        .ok_or("missing descendant PID")?
        .trim()
        .parse::<u32>()?;
    drop(process);
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match std::fs::read_to_string(format!("/proc/{pid}/status")) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => break,
            Ok(status)
                if status
                    .lines()
                    .any(|line| line.starts_with("State:") && line.contains('Z')) =>
            {
                break;
            }
            Err(error) => return Err(error.into()),
            Ok(_) => {}
        }
        assert!(
            Instant::now() < deadline,
            "descendant survived closing its pane"
        );
        thread::sleep(Duration::from_millis(2));
    }
    Ok(())
}
