// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

mod caps;
mod encode;
mod input;
mod session;

#[cfg(unix)]
pub use caps::probe_fd;
pub use caps::{
    Caps, ColorDepth, DecModes, GRAPHEME_ENABLE, GraphicsProtocol, Hyperlinks, KeyboardProtocol,
    Multiplexer, PROBE_BUDGET, PrivateMode, ProbeEnv, ProbeReply, Protocols, QUERIES,
    RecordedTerminal, TermIdentity, WidthPolicy,
};
pub use encode::{ColorMode, Encoder};
pub use input::{
    Event, Focus, KeyCode, KeyEvent, KeyKind, Modifiers, MouseButton, MouseEvent, MouseKind,
    MouseUnit, Parser, Paste, Scroll, fuzz_input_parser,
};
pub use session::{Session, SessionError, SessionOptions, SyncOutput};
