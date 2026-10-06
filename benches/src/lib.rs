// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("../README.md")]

mod alloc;
/// Workloads with known costs, used to check the harness itself. They stand in for
/// real scenes until lunatui has code to measure, and the harness self-test checks
/// the counts each one documents.
pub mod calibration;
/// The frame budget (PRD §1.3): a 16 ms maximum on shared CI runners, gated on
/// every sample of every worst-case benchmark, and an 8 ms target reported on a
/// dedicated machine.
pub mod ceiling;
/// A plain-text table of benchmark metrics.
pub mod report;
mod write;

pub use alloc::{AllocStats, CountingAllocator};
pub use write::{CountingWriter, WriteStats};
