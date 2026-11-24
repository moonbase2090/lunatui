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
use lunatui_image::ImageSource;
use lunatui_runtime::{App, ControlFlow, RenderFrame, RunOptions, is_quit, run};
use lunatui_term::{Event, PtyProcess, PtySize};

const CHILD: &str = "LUNATUI_GRAPHITE_LOG";
const QUERIES_END: &[u8] = b"\x1b[c";
const PLACED: &[u8] = b"a=p";

type Test = Result<(), Box<dyn std::error::Error>>;

struct Painter {
    log: PathBuf,
    logged: bool,
}

impl Painter {
    fn note(&self, line: &str) -> bool {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log)
            .and_then(|mut file| file.write_all(line.as_bytes()))
            .is_ok()
    }
}

impl App for Painter {
    fn update(&mut self, event: &Event) -> ControlFlow {
        if is_quit(event) {
            ControlFlow::Break
        } else {
            ControlFlow::Continue
        }
    }

    fn render(&mut self, _grid: &mut Grid, _area: Rect, _clusters: &mut ClusterTable) {}

    fn render_frame(&mut self, frame: &mut RenderFrame<'_>, _area: Rect) {
        let mode = frame.mode();
        let (_, _, images) = frame.parts();
        let area = Rect::new(1, 1, 4, 2);
        let submitted = images.pixel_target(area).is_some_and(|target| {
            let size = target.size();
            let pixels = vec![200_u8; (size.width() * size.height() * 4) as usize];
            ImageSource::rgba8(size.width(), size.height(), pixels.into_boxed_slice())
                .is_ok_and(|source| images.submit_raster(&source, area, 0))
        });
        if !self.logged {
            self.logged = self.note(&format!("{mode:?} submitted={submitted}\n"));
        }
    }
}

#[test]
fn child_app() -> io::Result<()> {
    let Some(log) = std::env::var_os(CHILD) else {
        return Ok(());
    };
    run(
        Painter {
            log: log.into(),
            logged: false,
        },
        RunOptions::new(40, 10),
    )
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn pump(process: &mut PtyProcess, output: &mut Vec<u8>) -> io::Result<()> {
    let mut buffer = [0_u8; 8192];
    process.flush_input()?;
    match process.read(&mut buffer) {
        Ok(count) => output.extend_from_slice(&buffer[..count]),
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
        Err(error) => return Err(error),
    }
    thread::sleep(Duration::from_millis(1));
    Ok(())
}

fn session(
    probe_reply: &[u8],
    name: &str,
) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    let log = std::env::temp_dir().join(format!(
        "lunatui-graphite-{name}-{}.log",
        std::process::id()
    ));
    let _ = fs::remove_file(&log);
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args(["child_app", "--exact", "--test-threads=1", "--nocapture"])
        .env(CHILD, &log);
    let mut process = PtyProcess::spawn(command, PtySize::new(40, 10)?)?;
    let mut output = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !contains(&output, QUERIES_END) {
        pump(&mut process, &mut output)?;
        if Instant::now() >= deadline {
            process.terminate()?;
            return Err("the app never probed the terminal".into());
        }
    }
    process.queue_input(probe_reply)?;
    let probed = output.len();
    while fs::metadata(&log).map_or(true, |meta| meta.len() == 0) {
        pump(&mut process, &mut output)?;
        if Instant::now() >= deadline {
            process.terminate()?;
            return Err("the app drew no frame".into());
        }
    }
    let settle = Instant::now() + Duration::from_millis(300);
    while Instant::now() < settle {
        pump(&mut process, &mut output)?;
    }
    process.queue_input(b"q")?;
    while process.try_wait()?.is_none() {
        pump(&mut process, &mut output)?;
        if Instant::now() >= deadline {
            process.terminate()?;
            return Err("the app did not quit".into());
        }
    }
    let notes = fs::read_to_string(&log)?;
    let _ = fs::remove_file(&log);
    Ok((output.split_off(probed), notes))
}

#[test]
fn a_kitty_terminal_with_a_cell_size_draws_graphite_and_places_images() -> Test {
    let (output, notes) = session(b"\x1b_Gi=31;OK\x1b\\\x1b[6;16;8t\x1b[?62c", "kitty")?;
    assert_eq!(notes, "Graphite submitted=true\n");
    assert!(contains(&output, PLACED), "the image is placed on screen");
    Ok(())
}

#[test]
fn a_kitty_terminal_without_a_cell_size_draws_cells() -> Test {
    let (output, notes) = session(b"\x1b_Gi=31;OK\x1b\\\x1b[?62c", "unsized")?;
    assert_eq!(notes, "Cell submitted=false\n");
    assert!(!contains(&output, PLACED), "no image is placed");
    Ok(())
}

#[test]
fn a_cell_only_terminal_draws_cells() -> Test {
    let (output, notes) = session(b"\x1b[6;16;8t\x1b[?62c", "cells")?;
    assert_eq!(notes, "Cell submitted=false\n");
    assert!(!contains(&output, PLACED), "no image is placed");
    Ok(())
}
