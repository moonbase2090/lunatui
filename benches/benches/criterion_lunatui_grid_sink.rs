// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::hint::black_box;
use std::io;

use criterion::{Criterion, criterion_group, criterion_main};
use lunatui_bench::scenes::lunatui::encode::EncodeFrames;
use lunatui_prismattyc::{HostRenderer, PrismattycBackend};
use lunatui_term::{ColorMode, Encoder};

const WIDTH: u16 = 200;
const HEIGHT: u16 = 60;

struct TouchCells;

impl HostRenderer for TouchCells {
    fn present_frame(&mut self, frame: &lunatui_prismattyc::FrameDiff<'_>) -> io::Result<()> {
        for run in frame.runs {
            let row = run.row;
            for col in run.col..run.col + run.width {
                black_box(frame.next.cell_bits(col, row));
            }
        }
        Ok(())
    }
}

fn lunatui_grid_sink_bench(c: &mut Criterion) {
    let frames = EncodeFrames::new(WIDTH, HEIGHT);
    let mut sink = PrismattycBackend::new(WIDTH, HEIGHT, TouchCells);
    let mut encoder = Encoder::new();
    let mut group = c.benchmark_group("lunatui_grid_sink_200x60");
    group.bench_function("present_scroll", |b| {
        b.iter(|| {
            sink.set_previous(frames.tick.clone());
            black_box(
                sink.present(black_box(&frames.scroll), black_box(&frames.clusters))
                    .ok(),
            );
        });
    });
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
    group.finish();
}

criterion_group!(benches, lunatui_grid_sink_bench);
criterion_main!(benches);
