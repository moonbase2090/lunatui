// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Instruction-count benchmarks of the calibration workloads (gungraun, PRD §6.1).
//!
//! Needs Linux, valgrind and a matching `gungraun-runner`. Run with
//! `cargo xtask bench`, which skips these when valgrind is missing.

#![allow(
    missing_docs,
    reason = "gungraun's benchmark macros generate undocumented public items"
)]

use std::hint::black_box;
use std::io;

use gungraun::{library_benchmark, library_benchmark_group, main};
use lunatui_bench::calibration::{LARGE_FRAME_BYTES, frame, grow_buffer, write_frame};

#[library_benchmark]
#[bench::large(LARGE_FRAME_BYTES)]
fn bench_frame(len: usize) -> Vec<u8> {
    black_box(frame(black_box(len)))
}

#[library_benchmark]
#[bench::large(frame(LARGE_FRAME_BYTES), Vec::with_capacity(LARGE_FRAME_BYTES))]
fn bench_write_frame(buffer: Vec<u8>, mut out: Vec<u8>) -> io::Result<Vec<u8>> {
    write_frame(&mut out, black_box(&buffer))?;
    Ok(out)
}

#[library_benchmark]
#[bench::small(1024)]
fn bench_grow_buffer(count: usize) -> Vec<u8> {
    black_box(grow_buffer(black_box(count)))
}

library_benchmark_group!(
    name = calibration,
    benchmarks = [bench_frame, bench_write_frame, bench_grow_buffer]
);

main!(library_benchmark_groups = calibration);
