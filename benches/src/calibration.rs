// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Workloads with known costs, used to check the harness itself.
//!
//! They stand in for real scenes until lunatui has code to measure (PR 03
//! adds the ratatui baseline scenes). Each documents the counts it must
//! produce, and the harness self-test checks them.

use std::io::{self, Write};

/// Size of the large truecolor frame in PRD §1.3 (68 KB).
pub const LARGE_FRAME_BYTES: usize = 68 * 1024;

/// Builds a frame buffer of `len` bytes.
///
/// Expected cost: exactly one allocation of `len` bytes (none when `len` is 0).
#[must_use]
pub fn frame(len: usize) -> Vec<u8> {
    vec![b'x'; len]
}

/// Writes `frame` to `out` in one `write_all`.
///
/// Expected cost into an [`io::sink`]: zero allocations, one `write()` call,
/// and `frame.len()` bytes.
///
/// # Errors
///
/// Returns any error from `out`.
pub fn write_frame<W: Write>(out: &mut W, frame: &[u8]) -> io::Result<()> {
    out.write_all(frame)
}

/// Pushes `count` bytes one at a time into an empty `Vec`.
///
/// Expected cost: one allocation, then a reallocation each time the vector
/// outgrows its capacity.
#[must_use]
pub fn grow_buffer(count: usize) -> Vec<u8> {
    let mut buffer = Vec::new();
    #[expect(
        clippy::same_item_push,
        reason = "growing one push at a time is the workload being measured"
    )]
    for _ in 0..count {
        buffer.push(b'x');
    }
    buffer
}
