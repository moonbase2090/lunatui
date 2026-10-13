// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

mod anim;
#[cfg(feature = "app")]
mod app;
#[cfg(feature = "app")]
mod focus;
#[cfg(feature = "app")]
mod hit;
mod input;
#[cfg(feature = "app")]
mod options;
#[cfg(feature = "app")]
mod run;
mod sched;

pub use anim::{Clock, Ease, Spring, Tween};
#[cfg(feature = "app")]
pub use app::{App, ControlFlow};
#[cfg(feature = "app")]
pub use focus::{FocusDirection, FocusManager};
#[cfg(feature = "app")]
pub use hit::{HitId, HitMap};
pub use input::Input;
#[cfg(feature = "app")]
pub use options::RunOptions;
#[cfg(feature = "app")]
pub use run::{is_quit, render_once, run};
pub use sched::{SchedulePoll, Scheduler, State};
