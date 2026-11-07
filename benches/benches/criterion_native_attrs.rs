// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use lunatui_core::{
    ChangedRun, ClusterTable, Grid, NativeAttrs, Style, diff_runs, diff_runs_into,
    diff_runs_visible_into,
};

const WIDTH: u16 = 200;
const HEIGHT: u16 = 60;

fn text_frame(clusters: &mut ClusterTable) -> Grid {
    let mut grid = Grid::new(WIDTH, HEIGHT);
    let line = "native alpha and sub-cell offsets over a dashboard row ".repeat(4);
    for row in 0..HEIGHT {
        grid.put_str(0, row, Style::new(), &line[..usize::from(WIDTH)], clusters);
    }
    grid
}

fn tenth_translucent(grid: &mut Grid) {
    let kinds = [
        NativeAttrs::new(255, 128, 0, 0),
        NativeAttrs::new(255, 64, 0, 0),
        NativeAttrs::new(200, 255, 0, 0),
        NativeAttrs::new(255, 255, 3, -2),
        NativeAttrs::new(255, 255, -5, 4),
    ];
    for row in 0..HEIGHT {
        for col in (0..WIDTH).filter(|col| (col + row) % 10 == 0) {
            let kind = kinds[usize::from(col / 10) % kinds.len()];
            grid.set_native(col, row, kind);
        }
    }
}

fn paint(c: &mut Criterion) {
    let mut clusters = ClusterTable::new();
    let mut group = c.benchmark_group("native_attrs_paint_200x60");
    group.bench_function("opaque", |b| {
        b.iter(|| black_box(text_frame(&mut clusters)));
    });
    group.bench_function("tenth_translucent_or_offset", |b| {
        b.iter(|| {
            let mut grid = text_frame(&mut clusters);
            tenth_translucent(&mut grid);
            black_box(grid)
        });
    });
    group.finish();
}

fn seed_and_diff(c: &mut Criterion) {
    let mut clusters = ClusterTable::new();
    let mut previous = text_frame(&mut clusters);
    tenth_translucent(&mut previous);
    let mut plain_previous = text_frame(&mut clusters);
    plain_previous.clear_dirty();
    let mut group = c.benchmark_group("native_attrs_frame_200x60");
    group.bench_function("seed_native_from", |b| {
        b.iter(|| {
            let mut next = Grid::new(WIDTH, HEIGHT);
            next.seed_native_from(&previous);
            black_box(next)
        });
    });
    group.bench_function("next_frame_seeded_and_diffed", |b| {
        b.iter(|| {
            let mut next = text_frame(&mut clusters);
            next.seed_native_from(&previous);
            tenth_translucent(&mut next);
            black_box(diff_runs(&previous, &next))
        });
    });
    group.bench_function("next_frame_opaque_and_diffed", |b| {
        b.iter(|| {
            let next = text_frame(&mut clusters);
            black_box(diff_runs(&plain_previous, &next))
        });
    });
    group.finish();
}

fn shifted_frame(clusters: &mut ClusterTable) -> Grid {
    let mut grid = Grid::new(WIDTH, HEIGHT);
    let line = "native alpha and sub-cell offsets over a dashboard row ".repeat(4);
    for row in 0..HEIGHT {
        let start = usize::from(row % 7) + 1;
        grid.put_str(
            0,
            row,
            Style::new(),
            &line[start..start + usize::from(WIDTH)],
            clusters,
        );
    }
    grid
}

fn visible_diff(c: &mut Criterion) {
    let mut clusters = ClusterTable::new();
    let mut plain = text_frame(&mut clusters);
    plain.clear_dirty();
    let mut tinted = text_frame(&mut clusters);
    tinted.seed_native_from(&plain);
    tenth_translucent(&mut tinted);
    let shifted = shifted_frame(&mut clusters);
    let mut runs: Vec<ChangedRun> = Vec::new();
    let mut group = c.benchmark_group("native_attrs_visible_diff_200x60");
    group.bench_function("attribute_only_plain_diff", |b| {
        b.iter(|| {
            diff_runs_into(&plain, &tinted, &mut runs);
            black_box(runs.len())
        });
    });
    group.bench_function("attribute_only_visible_diff", |b| {
        b.iter(|| {
            diff_runs_visible_into(&plain, &tinted, &mut runs);
            black_box(runs.len())
        });
    });
    group.bench_function("no_attributes_plain_diff", |b| {
        b.iter(|| {
            diff_runs_into(&plain, &shifted, &mut runs);
            black_box(runs.len())
        });
    });
    group.bench_function("no_attributes_visible_diff", |b| {
        b.iter(|| {
            diff_runs_visible_into(&plain, &shifted, &mut runs);
            black_box(runs.len())
        });
    });
    group.finish();
}

criterion_group!(benches, paint, seed_and_diff, visible_diff);
criterion_main!(benches);
