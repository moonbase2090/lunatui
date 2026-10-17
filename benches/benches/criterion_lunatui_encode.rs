// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use lunatui_bench::scenes::lunatui::encode::EncodeFrames;
use lunatui_term::{ColorMode, Encoder};

const WIDTH: u16 = 200;
const HEIGHT: u16 = 60;

fn lunatui_encode_bench(c: &mut Criterion) {
    let frames = EncodeFrames::new(WIDTH, HEIGHT);
    let mut encoder = Encoder::new();
    let mut group = c.benchmark_group("lunatui_encode_200x60");
    group.bench_function("encode_scroll", |b| {
        b.iter(|| {
            black_box(encoder.encode_diff(
                black_box(&frames.tick),
                black_box(&frames.scroll),
                black_box(&frames.clusters),
                ColorMode::TrueColor,
            ));
        });
    });
    group.bench_function("encode_full", |b| {
        b.iter(|| {
            black_box(encoder.encode_diff(
                black_box(&frames.empty),
                black_box(&frames.scroll),
                black_box(&frames.clusters),
                ColorMode::TrueColor,
            ));
        });
    });
    group.finish();
}

criterion_group!(benches, lunatui_encode_bench);
criterion_main!(benches);
