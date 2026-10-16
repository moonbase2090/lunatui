// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

mod caps;
mod encode;
mod input;
mod session;

/// Windows console VT output and crossterm input (PR 27).
#[cfg(windows)]
pub mod sys;

#[cfg(unix)]
pub use caps::probe_fd;
pub use caps::{
    Caps, ColorDepth, DecModes, GRAPHEME_ENABLE, GraphicsProtocol, Hyperlinks, KeyboardProtocol,
    Multiplexer, PROBE_BUDGET, PrivateMode, ProbeEnv, ProbeReply, Protocols, QUERIES,
    RecordedTerminal, TermIdentity, WidthPolicy,
};
pub use encode::{ColorMode, Encoder, write_cursor_state};
pub use input::{
    Event, Focus, KeyCode, KeyEvent, KeyKind, Modifiers, MouseButton, MouseEvent, MouseKind,
    MouseUnit, Parser, Paste, Scroll, fuzz_input_parser,
};
pub use session::{MouseTracking, Session, SessionError, SessionOptions, SyncOutput};
