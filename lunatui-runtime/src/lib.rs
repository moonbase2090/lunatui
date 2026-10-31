// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

mod anim;
#[cfg(feature = "app")]
mod app;
#[cfg(feature = "app")]
mod focus;
#[cfg(feature = "image-presenter")]
mod frame;
#[cfg(feature = "app")]
mod hit;
mod input;
/// Command registry for keyboard shortcuts. The shortcut overlay and command palette render from it.
#[cfg(feature = "keymap")]
pub mod keymap;
#[cfg(all(feature = "app", any(unix, windows)))]
mod options;
#[cfg(all(feature = "app", any(unix, windows)))]
mod run;
mod sched;

pub use anim::{Clock, Ease, Spring, Tween};
#[cfg(feature = "motion")]
pub use anim::{
    DampedSpring, EMPHASIS, FramePacer, MAX_DURATION, MICRO, MotionPolicy, STANDARD, Transition,
};
#[cfg(feature = "app")]
pub use app::{App, ControlFlow, is_quit};
#[cfg(feature = "app")]
pub use focus::{FocusDirection, FocusManager};
#[cfg(feature = "image-presenter")]
pub use frame::RenderFrame;
#[cfg(feature = "app")]
pub use hit::{HitId, HitMap};
pub use input::Input;
#[cfg(all(feature = "app", any(unix, windows)))]
pub use options::RunOptions;
#[cfg(all(feature = "app", any(unix, windows)))]
pub use run::{render_once, run};
pub use sched::{SchedulePoll, Scheduler, State};
