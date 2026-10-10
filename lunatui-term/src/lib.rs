// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("../README.md")]

mod caps;
mod encode;
mod session;

#[cfg(unix)]
pub use caps::probe_fd;
pub use caps::{
    Caps, ColorDepth, DecModes, GRAPHEME_ENABLE, GraphicsProtocol, Hyperlinks, KeyboardProtocol,
    Multiplexer, PROBE_BUDGET, PrivateMode, ProbeEnv, ProbeReply, Protocols, QUERIES,
    RecordedTerminal, TermIdentity, WidthPolicy,
};
pub use encode::{ColorMode, Encoder};
pub use session::{Session, SessionError, SessionOptions, SyncOutput};
