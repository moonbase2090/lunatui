// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

mod input;
mod sched;

pub use input::Input;
pub use sched::{SchedulePoll, Scheduler, State};
