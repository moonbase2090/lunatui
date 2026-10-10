// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("../README.md")]

mod fuzz;
mod oracle;

pub use fuzz::{fuzz_bytes, fuzz_campaign};
pub use oracle::{Oracle, OracleFailure, apply_diff, diff_matches};
