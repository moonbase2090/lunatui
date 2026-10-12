// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

mod app;
mod input;
mod options;
mod run;
mod sched;

pub use app::{App, ControlFlow};
pub use input::Input;
pub use options::RunOptions;
pub use run::{is_quit, render_once, run};
pub use sched::{SchedulePoll, Scheduler, State};
