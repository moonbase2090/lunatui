// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Harness self-test: the counters must report the documented cost of every
//! calibration workload.
//!
//! This runs without the libtest harness (`harness = false`), because the
//! allocation counts are process-wide and libtest's own threads would
//! otherwise add to them.

use std::io;

use lunatui_bench::calibration::{LARGE_FRAME_BYTES, frame, grow_buffer, write_frame};
use lunatui_bench::{AllocStats, CountingAllocator, CountingWriter, WriteStats};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

fn main() -> io::Result<()> {
    frame_allocates_once();
    dropping_a_frame_deallocates_once();
    write_frame_writes_once_without_allocating()?;
    grow_buffer_reallocates_as_it_grows();
    println!("harness self-test: ok");
    Ok(())
}

fn frame_allocates_once() {
    let (buffer, stats) = ALLOC.measure(|| frame(4096));
    let expected = AllocStats {
        allocations: 1,
        reallocations: 0,
        deallocations: 0,
        bytes_allocated: 4096,
    };
    assert_eq!(stats, expected, "frame(4096)");
    drop(buffer);
}

fn dropping_a_frame_deallocates_once() {
    let buffer = frame(4096);
    let ((), stats) = ALLOC.measure(|| drop(buffer));
    assert_eq!(stats.deallocations, 1, "drop(frame)");
    assert_eq!(stats.allocations, 0, "drop(frame)");
}

fn write_frame_writes_once_without_allocating() -> io::Result<()> {
    let buffer = frame(LARGE_FRAME_BYTES);
    let mut out = CountingWriter::new(io::sink());
    let (result, stats) = ALLOC.measure(|| write_frame(&mut out, &buffer));
    result?;

    assert_eq!(stats, AllocStats::default(), "write_frame allocations");
    let expected = WriteStats {
        writes: 1,
        bytes: LARGE_FRAME_BYTES,
        flushes: 0,
    };
    assert_eq!(out.stats(), expected, "write_frame writes");
    Ok(())
}

fn grow_buffer_reallocates_as_it_grows() {
    let (buffer, stats) = ALLOC.measure(|| grow_buffer(1024));
    assert_eq!(stats.allocations, 1, "grow_buffer(1024) allocations");
    assert!(stats.reallocations > 0, "grow_buffer(1024) reallocations");
    assert_eq!(buffer.len(), 1024);
}
