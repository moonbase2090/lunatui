// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

#[cfg(feature = "app")]
mod app;
mod input;
#[cfg(feature = "app")]
mod options;
#[cfg(feature = "app")]
mod run;
mod sched;

#[cfg(feature = "app")]
pub use app::{App, ControlFlow};
pub use input::Input;
#[cfg(feature = "app")]
pub use options::RunOptions;
#[cfg(feature = "app")]
pub use run::{is_quit, render_once, run};
pub use sched::{SchedulePoll, Scheduler, State};
