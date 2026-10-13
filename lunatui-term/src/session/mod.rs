// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;

#[cfg(not(any(unix, windows)))]
mod stub;

#[cfg(unix)]
pub use unix::{MouseTracking, Session, SessionError, SessionOptions, SyncOutput};

#[cfg(windows)]
pub use windows::{MouseTracking, Session, SessionError, SessionOptions, SyncOutput};

#[cfg(not(any(unix, windows)))]
pub use stub::{MouseTracking, Session, SessionError, SessionOptions, SyncOutput};
