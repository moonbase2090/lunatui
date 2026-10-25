// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::env;
use std::path::Path;
use std::process::{Command, ExitCode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BenchStep {
    target: &'static str,
    feature: Option<&'static str>,
    after: &'static [&'static str],
}

impl BenchStep {
    fn cargo_args(self) -> Vec<&'static str> {
        let mut args = vec!["bench", "--package", "lunatui-bench"];
        if let Some(feature) = self.feature {
            args.extend(["--features", feature]);
        }
        args.extend(["--bench", self.target]);
        if !self.after.is_empty() {
            args.push("--");
            args.extend(self.after);
        }
        args
    }
}

pub(crate) fn run() -> ExitCode {
    let valgrind = on_path("valgrind");
    if !valgrind {
        println!(
            "xtask: valgrind not found, skipping gungraun benchmarks (they need Linux and valgrind)"
        );
    }
    for step in bench_plan(valgrind) {
        println!("xtask: cargo {}", step.cargo_args().join(" "));
        if let Err(message) = run_cargo(&step.cargo_args()) {
            eprintln!("xtask: {message}");
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

fn bench_plan(valgrind: bool) -> Vec<BenchStep> {
    let mut plan = vec![
        BenchStep {
            target: "metrics",
            feature: None,
            after: &[],
        },
        BenchStep {
            target: "ceiling",
            feature: None,
            after: &[],
        },
        BenchStep {
            target: "criterion_calibration",
            feature: None,
            after: &[],
        },
        BenchStep {
            target: "widget_suite",
            feature: None,
            after: &["--gate"],
        },
    ];
    if valgrind {
        plan.push(BenchStep {
            target: "gungraun_calibration",
            feature: Some("gungraun"),
            after: &[],
        });
    }
    plan
}

fn run_cargo(args: &[&str]) -> Result<(), String> {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let status = Command::new(&cargo)
        .args(args)
        .status()
        .map_err(|error| format!("could not run cargo: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("`cargo {}` failed with {status}", args.join(" ")))
    }
}

fn on_path(program: &str) -> bool {
    env::var_os("PATH").is_some_and(|paths| {
        env::split_paths(&paths).any(|dir| Path::new(&dir).join(program).is_file())
    })
}

#[cfg(test)]
mod tests {
    use super::{BenchStep, bench_plan};

    const METRICS: BenchStep = BenchStep {
        target: "metrics",
        feature: None,
        after: &[],
    };
    const CEILING: BenchStep = BenchStep {
        target: "ceiling",
        feature: None,
        after: &[],
    };
    const CRITERION: BenchStep = BenchStep {
        target: "criterion_calibration",
        feature: None,
        after: &[],
    };
    const WIDGETS: BenchStep = BenchStep {
        target: "widget_suite",
        feature: None,
        after: &["--gate"],
    };
    const GUNGRAUN: BenchStep = BenchStep {
        target: "gungraun_calibration",
        feature: Some("gungraun"),
        after: &[],
    };

    #[test]
    fn plan_without_valgrind_skips_gungraun() {
        assert_eq!(bench_plan(false), [METRICS, CEILING, CRITERION, WIDGETS]);
    }

    #[test]
    fn plan_with_valgrind_ends_with_gungraun() {
        assert_eq!(
            bench_plan(true),
            [METRICS, CEILING, CRITERION, WIDGETS, GUNGRAUN]
        );
    }

    #[test]
    fn widget_suite_step_passes_the_gate_flag() {
        assert_eq!(
            WIDGETS.cargo_args(),
            [
                "bench",
                "--package",
                "lunatui-bench",
                "--bench",
                "widget_suite",
                "--",
                "--gate",
            ]
        );
    }

    #[test]
    fn gungraun_step_enables_its_feature() {
        assert_eq!(
            GUNGRAUN.cargo_args(),
            [
                "bench",
                "--package",
                "lunatui-bench",
                "--features",
                "gungraun",
                "--bench",
                "gungraun_calibration",
            ]
        );
    }
}
