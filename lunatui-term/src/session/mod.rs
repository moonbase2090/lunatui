// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#[cfg(unix)]
mod unix;

#[cfg(unix)]
pub use unix::{Session, SessionError, SessionOptions, SyncOutput};

#[cfg(not(unix))]
mod stub;

#[cfg(not(unix))]
pub use stub::{Session, SessionError, SessionOptions, SyncOutput};
