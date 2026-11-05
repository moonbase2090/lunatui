// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

mod anim;
#[cfg(feature = "app")]
mod app;
#[cfg(feature = "effects")]
/// Composable cell effects: fade, dissolve, sweep, slide and HSL shift.
///
/// An [`effect::Effect`] is a pure function of elapsed time. Render the widgets into a
/// grid as usual, then call [`effect::Effect::render`] with the time since the effect
/// started. The same elapsed time always produces the same cells, so frames are
/// reproducible and a golden diff is exact. Combinators run effects in
/// [`effect::Effect::sequence`] or [`effect::Effect::parallel`].
///
/// Every leaf has a strength that runs from 0 (the grid untouched) to 1 (fully
/// applied) along its delay, duration and easing. [`effect::Effect::reversed`] flips
/// the strength, which turns a fade-out into a fade-in. Colors blend in `OKLab`. A
/// cell is rewritten only while the strength is above zero, so theme tokens and
/// the focus and disabled colors a widget chose survive until an effect needs
/// their RGB value.
pub mod effect;
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
#[cfg(feature = "app")]
mod layer;
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
#[cfg(feature = "accessibility")]
pub use hit::AccessibilityRole;
#[cfg(feature = "app")]
pub use hit::{HitId, HitMap};
pub use input::Input;
#[cfg(feature = "app")]
pub use layer::{Layer, LayerId, LayerStack, PressRoute};
#[cfg(all(feature = "app", any(unix, windows)))]
pub use options::RunOptions;
#[cfg(all(feature = "app", any(unix, windows)))]
pub use run::{render_once, run};
pub use sched::{SchedulePoll, Scheduler, State};
