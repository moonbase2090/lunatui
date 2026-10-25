// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::cell::RefCell;
use std::io::{self, Write};
use std::process::Command;
use std::rc::Rc;

use lunatui_bench::widget_suite::{
    DEFAULT_MAX_RATIO, gate_failures, parse_max_ratio, pin_color_output,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Block;
use ratatui::{Terminal, TerminalOptions, Viewport};

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn max_ratio_defaults_when_absent() {
    assert_eq!(parse_max_ratio(&args(&["--gate"])), Ok(DEFAULT_MAX_RATIO));
}

#[test]
fn max_ratio_accepts_a_finite_positive_number() {
    assert_eq!(parse_max_ratio(&args(&["--max-ratio", "1.25"])), Ok(1.25));
}

#[test]
fn max_ratio_rejects_values_that_would_disable_the_gate() {
    for bad in ["NaN", "nan", "inf", "-inf", "0", "-1", "abc", ""] {
        assert!(
            parse_max_ratio(&args(&["--max-ratio", bad])).is_err(),
            "{bad:?} was accepted"
        );
    }
    assert!(parse_max_ratio(&args(&["--max-ratio"])).is_err());
}

#[test]
fn a_ratio_over_the_limit_fails() {
    let failures = gate_failures("x 80x24", 1.20, 1.05, 10.0, 10.0);
    assert_eq!(failures, ["x 80x24 frame ratio 1.20 > 1.05"]);
}

#[test]
fn a_ratio_that_is_not_a_number_fails() {
    assert_eq!(gate_failures("x", f64::NAN, 1.05, 1.0, 2.0).len(), 1);
    assert_eq!(gate_failures("x", f64::INFINITY, 1.05, 1.0, 2.0).len(), 1);
}

#[test]
fn more_allocations_than_ratatui_fail() {
    let failures = gate_failures("x", 0.50, 1.05, 11.0, 10.0);
    assert_eq!(failures, ["x allocations 11.0 > 10.0"]);
}

#[test]
fn a_case_at_the_limit_with_equal_allocations_passes() {
    assert_eq!(
        gate_failures("x", 1.05, 1.05, 10.0, 10.0),
        Vec::<String>::new()
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
