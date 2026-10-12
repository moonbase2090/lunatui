// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

mod fuzz;
mod oracle;

pub use fuzz::{fuzz_bytes, fuzz_campaign};
pub use oracle::{Oracle, OracleFailure, apply_diff, diff_matches};
