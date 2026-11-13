// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

mod caps;
mod encode;
mod input;
#[cfg(feature = "pixel-layer")]
mod pixel;
#[cfg(feature = "pty")]
mod pty;
mod session;
mod size;

/// Windows console VT output and crossterm input (PR 27).
#[cfg(windows)]
pub mod sys;

#[cfg(unix)]
pub use caps::probe_fd;
pub use caps::{
    Caps, CellPixels, ColorDepth, DecModes, GRAPHEME_ENABLE, GraphicsProtocol, Hyperlinks,
    KeyboardProtocol, Multiplexer, PROBE_BUDGET, PrivateMode, ProbeEnv, ProbeReply, Protocols,
    QUERIES, RecordedTerminal, TermIdentity, WidthPolicy,
};
pub use encode::{ColorMode, Encoder, write_cursor_state};
pub use input::{
    Event, Focus, KeyCode, KeyEvent, KeyKind, Modifiers, MouseButton, MouseEvent, MouseKind,
    MouseUnit, Parser, Paste, Scroll, fuzz_input_parser,
};
#[cfg(feature = "pixel-layer")]
pub use pixel::{PixelImage, PixelLayer, PixelMode};
pub use session::{MouseTracking, Session, SessionError, SessionOptions, SyncOutput};
pub use size::{SIZE_POLL, terminal_size, watch_size};

#[cfg(all(feature = "pty", unix))]
pub use pty::PtyProcess;
#[cfg(feature = "pty")]
pub use pty::PtySize;
