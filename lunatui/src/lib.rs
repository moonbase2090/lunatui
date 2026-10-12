// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

pub use lunatui_runtime::{App, ControlFlow, RunOptions, is_quit, render_once, run};

/// I/O errors from [`run`].
pub type Result<T = ()> = std::io::Result<T>;
