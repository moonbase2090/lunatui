// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::cell::RefCell;
use std::io::{self, Write};
use std::process::Command;
use std::rc::Rc;

use lunatui_bench::widget_suite::{gate_failures, pin_color_output};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Block;
use ratatui::{Terminal, TerminalOptions, Viewport};

#[test]
fn more_allocations_than_ratatui_fail() {
    assert_eq!(
        gate_failures("x", 11.0, 10.0),
        ["x allocations 11.0 > 10.0"]
    );
}

#[test]
fn equal_or_fewer_allocations_pass() {
    assert_eq!(gate_failures("x", 10.0, 10.0), Vec::<String>::new());
    assert_eq!(gate_failures("x", 0.0, 0.0), Vec::<String>::new());
    assert_eq!(gate_failures("x", 9.0, 10.0), Vec::<String>::new());
}

#[test]
fn invalid_allocation_counts_cannot_disable_the_gate() {
    assert_eq!(
        gate_failures("x", f64::NAN, 10.0),
        ["x allocations NaN > 10.0"]
    );
    assert_eq!(
        gate_failures("x", f64::INFINITY, 10.0),
        ["x allocations inf > 10.0"]
    );
}

#[derive(Clone, Default)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn colors_are_written_in_this_process() -> Result<(), String> {
    pin_color_output();
    let capture = Capture::default();
    let area = Rect::new(0, 0, 8, 3);
    let mut terminal = Terminal::with_options(
        CrosstermBackend::new(capture.clone()),
        TerminalOptions {
            viewport: Viewport::Fixed(area),
        },
    )
    .map_err(|error| format!("terminal setup failed: {error}"))?;
    let style = Style::new().fg(Color::Rgb(1, 2, 3)).bg(Color::Rgb(4, 5, 6));
    terminal
        .draw(|frame| {
            frame.render_widget(Block::bordered().border_style(style).style(style), area);
        })
        .map_err(|error| format!("draw failed: {error}"))?;
    let text = String::from_utf8_lossy(&capture.0.borrow()).into_owned();
    if !text.contains("38;2;1;2;3") {
        return Err(format!("no truecolor foreground in {text:?}"));
    }
    if !text.contains("48;2;4;5;6") {
        return Err(format!("no truecolor background in {text:?}"));
    }
    Ok(())
}

fn run_child(no_color: bool) -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let mut command = Command::new(exe);
    command.args([
        "--exact",
        "colors_are_written_in_this_process",
        "--nocapture",
    ]);
    if no_color {
        command.env("NO_COLOR", "1");
    } else {
        command.env_remove("NO_COLOR");
    }
    let Ok(output) = command.output() else {
        return false;
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    output.status.success() && stdout.contains("1 passed")
}

#[test]
fn colors_are_written_with_and_without_no_color() {
    assert!(run_child(false), "child without NO_COLOR failed");
    assert!(run_child(true), "child with NO_COLOR=1 failed");
}
