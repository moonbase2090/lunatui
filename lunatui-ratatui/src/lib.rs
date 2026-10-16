// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

mod backend;
mod convert;

pub use backend::LunatuiBackend;
pub use convert::{buffer_row_text, grid_from_buffer, map_color};
