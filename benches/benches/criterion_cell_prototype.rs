// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("../README.md")]

use std::hint::black_box;
use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};
use lunatui_bench::prototype::{Packer, changed_cells, changed_words, row_hashes};
use lunatui_bench::scenes::ratatui::dashboard::Frames;
use ratatui::layout::Rect;

fn configured() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_millis(300))
        .measurement_time(Duration::from_secs(1))
        .sample_size(30)
}

fn cell_diff(c: &mut Criterion) {
    for (width, height) in [(200_u16, 60_u16), (400, 120)] {
        let area = Rect::new(0, 0, width, height);
        let frames = Frames::new(area);
        let identical = frames.scroll.clone();
        let mut packer = Packer::new();
        let first = packer.pack(&frames.first);
        let tick = packer.pack(&frames.tick);
        let scroll = packer.pack(&frames.scroll);
        let columns = usize::from(width);
        let mut group = c.benchmark_group(format!("cell_diff/{width}x{height}"));
        for (name, previous, next) in [
            ("ratatui_identical", &frames.scroll, &identical),
            ("ratatui_tick", &frames.first, &frames.tick),
            ("ratatui_scroll", &frames.tick, &frames.scroll),
        ] {
            group.bench_function(name, |b| {
                b.iter(|| black_box(previous.diff_iter(next).count()));
            });
        }
        for (name, previous, next) in [
            ("packed_identical", &scroll, &scroll),
            ("packed_tick", &first, &tick),
            ("packed_scroll", &tick, &scroll),
        ] {
            group.bench_function(name, |b| {
                b.iter(|| black_box(changed_cells(black_box(previous), black_box(next), columns)));
            });
        }
        let [first_words, tick_words, scroll_words] = [&first, &tick, &scroll].map(|cells| {
            cells
                .iter()
                .map(|cell| cell.to_bits())
                .collect::<Vec<u128>>()
        });
        for (name, previous, next) in [
            ("words_identical", &scroll_words, &scroll_words),
            ("words_tick", &first_words, &tick_words),
            ("words_scroll", &tick_words, &scroll_words),
        ] {
            group.bench_function(name, |b| {
                b.iter(|| black_box(changed_words(black_box(previous), black_box(next), columns)));
            });
        }
        let mut hashes = Vec::with_capacity(usize::from(height));
        group.bench_function("packed_row_hashes", |b| {
            b.iter(|| {
                row_hashes(black_box(&scroll), columns, &mut hashes);
                black_box(&hashes);
            });
        });
        group.finish();
    }
}

criterion_group! {
    name = cell_prototype;
    config = configured();
    targets = cell_diff
}
criterion_main!(cell_prototype);
