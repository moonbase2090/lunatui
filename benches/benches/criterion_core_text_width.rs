// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use lunatui_core::simd::{ascii_display_width, ascii_display_width_scalar};
use lunatui_core::{cluster_display_width, text_display_width};
use unicode_segmentation::UnicodeSegmentation;

const ASCII_LOG_LINES: usize = 5000;

fn ascii_log_text() -> String {
    let line = "2026-10-16T19:00:00Z INFO  worker-3 completed batch id=0xdeadbeef duration=12ms\n";
    line.repeat(ASCII_LOG_LINES)
}

fn grapheme_display_width(text: &str) -> usize {
    UnicodeSegmentation::graphemes(text, true)
        .map(cluster_display_width)
        .sum()
}

fn mixed_ascii_run_text() -> String {
    let line = "2026-10-16T19:00:00Z INFO  worker-3 completed batch id=0xdeadbeef duration=12ms\n";
    let mut text = String::with_capacity((line.len() + '日'.len_utf8()) * ASCII_LOG_LINES);
    for _ in 0..ASCII_LOG_LINES {
        text.push_str(line);
        text.push('日');
    }
    text
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

fn mixed_ascii_run_width_bench(c: &mut Criterion) {
    let text = mixed_ascii_run_text();
    let mut group = c.benchmark_group("core_text_width_mixed_ascii_runs_5000");
    group.bench_function("before_grapheme_clusters", |b| {
        b.iter(|| black_box(grapheme_display_width(black_box(&text))));
    });
    group.bench_function("after_ascii_run_simd", |b| {
        b.iter(|| black_box(text_display_width(black_box(&text))));
    });
    group.finish();
}

criterion_group!(benches, ascii_log_width_bench, mixed_ascii_run_width_bench);
criterion_main!(benches);
