// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::io;

use criterion::{Criterion, criterion_group, criterion_main};
use lunatui_bench::scenes::lunatui::log;

const WIDTH: u16 = 400;
const HEIGHT: u16 = 120;

fn log_wrap_scroll(c: &mut Criterion) {
    let text = log::text(5000);
    let mut group = c.benchmark_group("lunatui_log");
    for scroll in [0_u16, 1000, 4000] {
        group.bench_function(format!("wrap_scroll_{scroll}"), |b| {
            b.iter(|| log::render(&text, scroll, WIDTH, HEIGHT));
        });
    }
    group.finish();
}

criterion_group!(benches, log_wrap_scroll);
criterion_main!(benches);
