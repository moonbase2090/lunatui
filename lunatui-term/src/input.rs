// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

mod keys;
mod mouse;
mod parser;
mod paste;

pub use keys::{KeyCode, KeyEvent, KeyKind, Modifiers};
pub use mouse::{MouseButton, MouseEvent, MouseKind, MouseUnit, Scroll};
pub use parser::{Event, Focus, Parser, fuzz_input_parser};
pub use paste::Paste;
