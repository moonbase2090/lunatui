// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

#[cfg(feature = "app")]
pub use lunatui_runtime::{App, ControlFlow, RunOptions, is_quit, render_once, run};

#[cfg(feature = "app")]
/// I/O errors from [`run`].
pub type Result<T = ()> = std::io::Result<T>;
