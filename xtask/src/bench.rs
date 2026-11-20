// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::env;
use std::path::Path;
use std::process::{Command, ExitCode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BenchStep {
    Cargo {
        target: &'static str,
        feature: Option<&'static str>,
        after: &'static [&'static str],
    },
    Python(&'static str),
}

impl BenchStep {
    fn args(self) -> Vec<&'static str> {
        match self {
            Self::Python(script) => vec![script],
            Self::Cargo {
                target,
                feature,
                after,
            } => {
                let mut args = vec!["bench", "--package", "lunatui-bench"];
                if let Some(feature) = feature {
                    args.extend(["--features", feature]);
                }
                args.extend(["--bench", target]);
                if !after.is_empty() {
                    args.push("--");
                    args.extend(after);
                }
                args
            }
        }
    }
}

pub(crate) fn run() -> ExitCode {
    let valgrind = on_path("valgrind");
    if !valgrind {
        println!(
            "xtask: valgrind not found; widget allocations are gated, instruction checks are skipped"
        );
    }
    for step in bench_plan(valgrind) {
        if let Err(message) = run_step(step) {
            eprintln!("xtask: {message}");
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

fn bench_plan(valgrind: bool) -> Vec<BenchStep> {
    let mut plan = vec![
        BenchStep::Cargo {
            target: "metrics",
            feature: None,
            after: &[],
        },
        BenchStep::Cargo {
            target: "ceiling",
            feature: None,
            after: &[],
        },
        BenchStep::Cargo {
            target: "criterion_calibration",
            feature: None,
            after: &[],
        },
        BenchStep::Cargo {
            target: "widget_suite",
            feature: Some("text-area"),
            after: &["--gate"],
        },
    ];
    if valgrind {
        plan.push(BenchStep::Python("scripts/test-widget-instructions.py"));
        plan.push(BenchStep::Python("scripts/check-widget-instructions.py"));
    }
    plan.push(BenchStep::Cargo {
        target: "criterion_widgets",
        feature: Some("text-area"),
        after: &["text-area"],
    });
    if valgrind {
        plan.push(BenchStep::Cargo {
            target: "gungraun_calibration",
            feature: Some("gungraun"),
            after: &[],
        });
    }
    plan
}

fn run_step(step: BenchStep) -> Result<(), String> {
    let program = match step {
        BenchStep::Cargo { .. } => env::var_os("CARGO").unwrap_or_else(|| "cargo".into()),
        BenchStep::Python(_) => "python3".into(),
    };
    let args = step.args();
    println!("xtask: {} {}", program.to_string_lossy(), args.join(" "));
    let status = Command::new(&program)
        .args(&args)
        .status()
        .map_err(|error| format!("could not run benchmark step: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "benchmark step `{}` failed with {status}",
            args.join(" ")
        ))
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

    const METRICS: BenchStep = BenchStep::Cargo {
        target: "metrics",
        feature: None,
        after: &[],
    };
    const CEILING: BenchStep = BenchStep::Cargo {
        target: "ceiling",
        feature: None,
        after: &[],
    };
    const CRITERION: BenchStep = BenchStep::Cargo {
        target: "criterion_calibration",
        feature: None,
        after: &[],
    };
    const WIDGETS: BenchStep = BenchStep::Cargo {
        target: "widget_suite",
        feature: Some("text-area"),
        after: &["--gate"],
    };
    const TEXTAREA_CRITERION: BenchStep = BenchStep::Cargo {
        target: "criterion_widgets",
        feature: Some("text-area"),
        after: &["text-area"],
    };
    const GUNGRAUN: BenchStep = BenchStep::Cargo {
        target: "gungraun_calibration",
        feature: Some("gungraun"),
        after: &[],
    };

    #[test]
    fn plan_without_valgrind_skips_gungraun() {
        assert_eq!(
            bench_plan(false),
            [METRICS, CEILING, CRITERION, WIDGETS, TEXTAREA_CRITERION]
        );
    }

    #[test]
    fn plan_with_valgrind_ends_with_gungraun() {
        assert_eq!(
            bench_plan(true),
            [
                METRICS,
                CEILING,
                CRITERION,
                WIDGETS,
                BenchStep::Python("scripts/test-widget-instructions.py"),
                BenchStep::Python("scripts/check-widget-instructions.py"),
                TEXTAREA_CRITERION,
                GUNGRAUN
            ]
        );
    }

    #[test]
    fn widget_suite_step_passes_the_gate_flag() {
        assert_eq!(
            WIDGETS.args(),
            [
                "bench",
                "--package",
                "lunatui-bench",
                "--features",
                "text-area",
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
            GUNGRAUN.args(),
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
