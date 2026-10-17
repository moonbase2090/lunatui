// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::hint::black_box;
use std::vec::Vec;

use criterion::{Criterion, criterion_group, criterion_main};
use lunatui_core::{
    Grid,
    simd::{self, collect_runs_in_row_scalar, collect_runs_in_row_simd},
};

const WIDTH: u16 = 200;
const HEIGHT: u16 = 60;

fn grids_with_sparse_changes() -> (Grid, Grid) {
    let mut previous = Grid::new(WIDTH, HEIGHT);
    let mut next = previous.clone();
    let mut clusters = lunatui_core::ClusterTable::new();
    let style = lunatui_core::Style::new();
    previous.put_str(10, 5, style, "seed", &mut clusters);
    next.put_str(10, 5, style, "seed", &mut clusters);
    for col in [0_u16, 40, 120, 199] {
        for row in [0_u16, 12, 30, 59] {
            next.put_str(col, row, style, "X", &mut clusters);
        }
    }
    (previous, next)
}

fn row_collect_bench(c: &mut Criterion) {
    let (previous, next) = grids_with_sparse_changes();
    let height = next.height();
    let width = next.width();
    let mut group = c.benchmark_group("core_row_collect_200x60");
    group.bench_function("scalar", |b| {
        b.iter(|| {
            let mut runs = Vec::new();
            for row in 0..height {
                runs.extend(collect_runs_in_row_scalar(&previous, &next, row, 0, width));
            }
            black_box(runs);
        });
    });
    group.bench_function("dispatched", |b| {
        b.iter(|| {
            let mut runs = Vec::new();
            for row in 0..height {
                runs.extend(collect_runs_in_row_simd(&previous, &next, row, 0, width));
            }
            black_box(runs);
        });
    });
    group.finish();
}

fn row_tail_first_mismatch(c: &mut Criterion) {
    let width = 200usize;
    let previous = vec![1_u128; width];
    let mut next = previous.clone();
    next[width - 1] = 2;
    let mut group = c.benchmark_group("core_first_mismatch_row200");
    group.bench_function("scalar", |b| {
        b.iter(|| black_box(simd::first_mismatch_scalar(&previous, &next)));
    });
    group.bench_function("dispatched", |b| {
        b.iter(|| black_box(simd::first_mismatch(&previous, &next)));
    });
    group.finish();
}

fn first_mismatch_bench(c: &mut Criterion) {
    let len = usize::from(WIDTH) * usize::from(HEIGHT);
    let previous = vec![0_u128; len];
    let mut next = previous.clone();
    for index in (0..len).step_by(97) {
        next[index] = index as u128 + 1;
    }
    let mut group = c.benchmark_group("core_first_mismatch_200x60");
    group.bench_function("scalar", |b| {
        b.iter(|| black_box(simd::first_mismatch_scalar(&previous, &next)));
    });
    group.bench_function("dispatched", |b| {
        b.iter(|| black_box(simd::first_mismatch(&previous, &next)));
    });
    group.finish();
}

criterion_group!(
    benches,
    row_collect_bench,
    row_tail_first_mismatch,
    first_mismatch_bench
);
criterion_main!(benches);
