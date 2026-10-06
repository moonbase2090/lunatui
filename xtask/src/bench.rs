// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::env;
use std::path::Path;
use std::process::{Command, ExitCode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BenchStep {
    target: &'static str,
    feature: Option<&'static str>,
}

impl BenchStep {
    fn cargo_args(self) -> Vec<&'static str> {
        let mut args = vec!["bench", "--package", "lunatui-bench"];
        if let Some(feature) = self.feature {
            args.extend(["--features", feature]);
        }
        args.extend(["--bench", self.target]);
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
        },
        BenchStep {
            target: "ceiling",
            feature: None,
        },
        BenchStep {
            target: "criterion_calibration",
            feature: None,
        },
    ];
    if valgrind {
        plan.push(BenchStep {
            target: "gungraun_calibration",
            feature: Some("gungraun"),
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
    };
    const CEILING: BenchStep = BenchStep {
        target: "ceiling",
        feature: None,
    };
    const CRITERION: BenchStep = BenchStep {
        target: "criterion_calibration",
        feature: None,
    };
    const GUNGRAUN: BenchStep = BenchStep {
        target: "gungraun_calibration",
        feature: Some("gungraun"),
    };

    #[test]
    fn plan_without_valgrind_skips_gungraun() {
        assert_eq!(bench_plan(false), [METRICS, CEILING, CRITERION]);
    }

    #[test]
    fn plan_with_valgrind_ends_with_gungraun() {
        assert_eq!(bench_plan(true), [METRICS, CEILING, CRITERION, GUNGRAUN]);
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
