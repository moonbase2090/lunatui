// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use lunatui_core::simd::{ascii_display_width, ascii_display_width_scalar};
use lunatui_core::text_display_width;

const ASCII_LOG_LINES: usize = 5000;

fn ascii_log_text() -> String {
    let line = "2026-10-16T19:00:00Z INFO  worker-3 completed batch id=0xdeadbeef duration=12ms\n";
    line.repeat(ASCII_LOG_LINES)
}

fn ascii_log_width_bench(c: &mut Criterion) {
    let text = ascii_log_text();
    let mut group = c.benchmark_group("core_text_width_ascii_log_5000");
    group.bench_function("scalar", |b| {
        b.iter(|| black_box(ascii_display_width_scalar(black_box(&text))));
    });
    group.bench_function("dispatched", |b| {
        b.iter(|| black_box(ascii_display_width(black_box(&text))));
    });
    group.bench_function("text_display_width", |b| {
        b.iter(|| black_box(text_display_width(black_box(&text))));
    });
    group.finish();
}

criterion_group!(benches, ascii_log_width_bench);
criterion_main!(benches);
