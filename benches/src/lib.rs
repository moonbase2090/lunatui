// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Benchmark harness for lunatui (PRD §6, §8 PR 02).
//!
//! It provides the counters every benchmark reports alongside time:
//!
//! - [`CountingAllocator`]: allocations and allocated bytes, installed as the
//!   `#[global_allocator]` of a bench or example binary.
//! - [`CountingWriter`]: `write()` calls and bytes written through any
//!   [`std::io::Write`].
//! - [`calibration`]: workloads with known costs, used to check the harness
//!   itself.
//! - [`ceiling`]: the 8 ms maximum frame-time gate (PRD §1.3).
//! - [`report`]: a plain-text metrics table.
//!
//! Run everything with `cargo xtask bench`.

mod alloc;
pub mod calibration;
pub mod ceiling;
pub mod report;
mod write;

pub use alloc::{AllocStats, CountingAllocator};
pub use write::{CountingWriter, WriteStats};
