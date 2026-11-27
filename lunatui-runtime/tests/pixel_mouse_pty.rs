// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]
#![cfg(all(unix, feature = "image-presenter"))]

use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_runtime::{App, ControlFlow, RunOptions, run};
use lunatui_term::{Event, KeyCode, PtyProcess, PtySize};

const CHILD: &str = "LUNATUI_POINTER_LOG";
const QUERIES_END: &[u8] = b"\x1b[c";
const PIXELS_ON: &[u8] = b"\x1b[?1016h";
const SGR_ON: &[u8] = b"\x1b[?1006h";

type Test = Result<(), Box<dyn std::error::Error>>;

struct Recorder {
    log: PathBuf,
}

impl App for Recorder {
    fn update(&mut self, event: &Event) -> ControlFlow {
        match event {
            Event::Mouse(mouse) => {
                let line = format!(
                    "{:?} {} {} {:?} {:?}\n",
                    mouse.kind,
                    mouse.column,
                    mouse.row,
                    mouse.unit,
                    mouse.inside.map(|inside| (inside.x(), inside.y()))
                );
                let written = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&self.log)
                    .and_then(|mut file| file.write_all(line.as_bytes()));
                if written.is_err() {
                    return ControlFlow::Break;
                }
                ControlFlow::Continue
            }
            Event::Key(key) if key.code == KeyCode::Char('q') => ControlFlow::Break,
            _ => ControlFlow::Continue,
        }
    }

    fn render(&mut self, _grid: &mut Grid, _area: Rect, _clusters: &mut ClusterTable) {}
}

#[test]
fn child_app() -> io::Result<()> {
    let Some(log) = std::env::var_os(CHILD) else {
        return Ok(());
    };
    run(Recorder { log: log.into() }, RunOptions::new(40, 10))
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn read_until(process: &mut PtyProcess, output: &mut Vec<u8>, wanted: &[u8]) -> io::Result<()> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut buffer = [0_u8; 4096];
    while !contains(output, wanted) {
        process.flush_input()?;
        match process.read(&mut buffer) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    String::from_utf8_lossy(output).into_owned(),
                ));
            }
            Ok(count) => output.extend_from_slice(&buffer[..count]),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(1));
            }
            Err(error) => return Err(error),
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                String::from_utf8_lossy(output).into_owned(),
            ));
        }
    }
    Ok(())
}

fn wait_exit(process: &mut PtyProcess) -> io::Result<bool> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut sink = [0_u8; 4096];
    loop {
        process.flush_input()?;
        if let Some(status) = process.try_wait()? {
            return Ok(status.success());
        }
        let _ = process.read(&mut sink);
        if Instant::now() >= deadline {
            process.terminate()?;
            return Ok(false);
        }
        thread::sleep(Duration::from_millis(2));
    }
}

fn session(
    probe_reply: &[u8],
    click: &[u8],
    name: &str,
) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    let log =
        std::env::temp_dir().join(format!("lunatui-pointer-{name}-{}.log", std::process::id()));
    let _ = fs::remove_file(&log);
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args(["child_app", "--exact", "--test-threads=1", "--nocapture"])
        .env(CHILD, &log);
    let mut process = PtyProcess::spawn(command, PtySize::new(40, 10)?)?;
    let mut output = Vec::new();
    read_until(&mut process, &mut output, QUERIES_END)?;
    process.queue_input(probe_reply)?;
    read_until(&mut process, &mut output, SGR_ON)?;
    process.queue_input(click)?;
    let deadline = Instant::now() + Duration::from_secs(10);
    while fs::metadata(&log).map_or(true, |meta| meta.len() == 0) {
        let mut buffer = [0_u8; 4096];
        process.flush_input()?;
        if let Ok(count) = process.read(&mut buffer) {
            output.extend_from_slice(&buffer[..count]);
        }
        if Instant::now() >= deadline {
            process.terminate()?;
            return Err("the app saw no mouse event".into());
        }
        thread::sleep(Duration::from_millis(2));
    }
    process.queue_input(b"q")?;
    let exited = wait_exit(&mut process)?;
    let events = fs::read_to_string(&log)?;
    let _ = fs::remove_file(&log);
    if !exited {
        return Err(format!("the app did not exit cleanly; events {events:?}").into());
    }
    Ok((output, events))
}

#[test]
fn a_terminal_that_reports_pixels_gets_mode_1016_and_the_app_gets_cells_with_the_offset() -> Test {
    let (output, events) = session(
        b"\x1b[?1016;2$y\x1b[6;16;8t\x1b[?62c",
        b"\x1b[<0;36;51M",
        "pixels",
    )?;
    assert!(
        contains(&output, PIXELS_ON),
        "the session asks for pixel reports"
    );
    assert_eq!(events, "Down(Left) 4 3 Cells Some((3, 2))\n");
    Ok(())
}

#[test]
fn a_terminal_without_mode_1016_keeps_cell_reports() -> Test {
    let (output, events) = session(
        b"\x1b[?1016;0$y\x1b[6;16;8t\x1b[?62c",
        b"\x1b[<0;5;4M",
        "cells",
    )?;
    assert!(
        !contains(&output, PIXELS_ON),
        "no pixel reports without mode 1016"
    );
    assert_eq!(events, "Down(Left) 4 3 Cells None\n");
    Ok(())
}

#[test]
fn a_terminal_that_gives_no_cell_size_keeps_cell_reports() -> Test {
    let (output, events) = session(b"\x1b[?1016;2$y\x1b[?62c", b"\x1b[<0;5;4M", "unsized")?;
    assert!(
        !contains(&output, PIXELS_ON),
        "no pixel reports without a cell size"
    );
    assert_eq!(events, "Down(Left) 4 3 Cells None\n");
    Ok(())
}
