// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_bench::widget_suite::{
    DEFAULT_MAX_RATIO, gate_failures, parse_max_ratio, pin_color_output,
};
use ratatui::crossterm::style::Colored;

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

#[test]
fn pinning_color_output_overrides_no_color() {
    Colored::set_ansi_color_disabled(true);
    pin_color_output();
    assert!(!Colored::ansi_color_disabled());
}
