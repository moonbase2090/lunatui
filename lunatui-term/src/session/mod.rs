// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#[cfg(unix)]
mod unix;

#[cfg(unix)]
pub use unix::{Session, SessionError, SessionOptions, SyncOutput};

#[cfg(not(unix))]
mod stub;

#[cfg(not(unix))]
pub use stub::{Session, SessionError, SessionOptions, SyncOutput};
