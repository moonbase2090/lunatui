// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::hint::{black_box, spin_loop};
use std::io;
use std::sync::atomic::{AtomicU8, Ordering};
use std::thread;

use lunatui_bench::calibration::{LARGE_FRAME_BYTES, frame, grow_buffer, write_frame};
use lunatui_bench::{AllocStats, CountingAllocator, CountingWriter, WriteStats};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

fn main() -> io::Result<()> {
    frame_allocates_once();
    dropping_a_frame_deallocates_once();
    write_frame_writes_once_without_allocating()?;
    grow_buffer_reallocates_as_it_grows();
    allocations_on_other_threads_are_not_measured();
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

static HELPER: AtomicU8 = AtomicU8::new(0);
const GO: u8 = 1;
const DONE: u8 = 2;

fn allocations_on_other_threads_are_not_measured() {
    let helper = thread::spawn(|| {
        while HELPER.load(Ordering::Acquire) != GO {
            spin_loop();
        }
        drop(black_box(Vec::<u8>::with_capacity(256)));
        HELPER.store(DONE, Ordering::Release);
    });
    let process_before = ALLOC.snapshot();
    let ((), stats) = ALLOC.measure(|| {
        HELPER.store(GO, Ordering::Release);
        while HELPER.load(Ordering::Acquire) != DONE {
            spin_loop();
        }
    });
    let process = ALLOC.snapshot().since(process_before);
    assert!(
        process.allocations >= 1 && process.bytes_allocated >= 256,
        "the helper thread allocated during the measure: {process:?}"
    );
    assert_eq!(
        stats,
        AllocStats::default(),
        "measured on the calling thread"
    );
    assert!(helper.join().is_ok(), "helper thread");
}
