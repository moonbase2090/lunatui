// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use lunatui_bench::calibration::{LARGE_FRAME_BYTES, frame, grow_buffer, write_frame};

fn calibration(c: &mut Criterion) {
    let mut group = c.benchmark_group("calibration");
    group.throughput(Throughput::Bytes(LARGE_FRAME_BYTES as u64));
    group.bench_function("frame_68k", |b| {
        b.iter(|| frame(black_box(LARGE_FRAME_BYTES)));
    });
    let large = frame(LARGE_FRAME_BYTES);
    let mut reused_presized_output = Vec::with_capacity(LARGE_FRAME_BYTES);
    group.bench_function("write_frame_68k", |b| {
        b.iter(|| {
            reused_presized_output.clear();
            write_frame(&mut reused_presized_output, black_box(&large))
        });
    });
    group.finish();

    c.bench_function("calibration/grow_buffer_1k", |b| {
        b.iter(|| grow_buffer(black_box(1024)));
    });
}

criterion_group!(benches, calibration);
criterion_main!(benches);
