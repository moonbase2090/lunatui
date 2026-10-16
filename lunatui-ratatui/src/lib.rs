// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

mod backend;
mod convert;
mod memo;
mod style;
mod text;
mod widget;

pub use backend::LunatuiBackend;
pub use convert::{buffer_row_text, grid_from_buffer, map_color};
pub use style::{color_from_ratatui, color_to_ratatui, style_from_ratatui, style_to_ratatui};
pub use text::{StyledLine, StyledSpan, StyledText};
pub use widget::{RatatuiStatefulWidget, RatatuiWidget};
