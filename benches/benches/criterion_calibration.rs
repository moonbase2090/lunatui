// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Wall-clock benchmarks of the calibration workloads (criterion, PRD §6.1).

use std::hint::black_box;
use std::io;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use lunatui_bench::calibration::{LARGE_FRAME_BYTES, frame, grow_buffer, write_frame};

fn calibration(c: &mut Criterion) {
    let mut group = c.benchmark_group("calibration");
    group.throughput(Throughput::Bytes(LARGE_FRAME_BYTES as u64));
    group.bench_function("frame_68k", |b| {
        b.iter(|| frame(black_box(LARGE_FRAME_BYTES)));
    });
    let large = frame(LARGE_FRAME_BYTES);
    group.bench_function("write_frame_68k", |b| {
        b.iter(|| write_frame(&mut io::sink(), black_box(&large)));
    });
    group.finish();

    c.bench_function("calibration/grow_buffer_1k", |b| {
        b.iter(|| grow_buffer(black_box(1024)));
    });
}

criterion_group!(benches, calibration);
criterion_main!(benches);
