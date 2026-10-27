// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

/// A Markdown parser.
#[cfg(feature = "markdown")]
pub mod markdown;
/// Virtualized row access for lists and tables.
pub mod source;
/// Word wrap and [`WrapCache`](wrap::WrapCache).
pub mod wrap;

pub use source::{RowBuilder, RowSource, SliceSource, VecSource};
