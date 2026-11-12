// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_bench::CountingAllocator;
use lunatui_bench::scenes::lunatui::dashboard as luna;
use lunatui_bench::scenes::ratatui::dashboard::{Model, advance_scripted};
use lunatui_core::{ClusterTable, Grid};
use lunatui_term::{ColorMode, Encoder};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

const WIDTH: u16 = 200;
const HEIGHT: u16 = 60;
const WARM_UP: u32 = 240;
const FRAMES: u32 = 240;
const MAX_RENDER_TOTAL: usize = 23_740;
const MAX_RENDER_FRAME: usize = 114;

fn per_frame() -> (usize, usize, usize) {
    let mut model = Model::new();
    let mut previous = Grid::new(WIDTH, HEIGHT);
    let mut next = Grid::new(WIDTH, HEIGHT);
    let mut clusters = ClusterTable::new();
    let mut encoder = Encoder::new();
    let mut render = 0;
    let mut worst = 0;
    let mut encode = 0;
    for tick in 0..WARM_UP + FRAMES {
        advance_scripted(&mut model, tick);
        let ((), drawn) =
            ALLOC.measure(|| luna::render(&model, WIDTH, HEIGHT, &mut next, &mut clusters));
        let (_, diffed) = ALLOC.measure(|| {
            encoder
                .encode_diff(&previous, &next, &clusters, ColorMode::TrueColor)
                .len()
        });
        if tick >= WARM_UP {
            render += drawn.allocations;
            worst = worst.max(drawn.allocations);
            encode += diffed.allocations;
        }
        std::mem::swap(&mut previous, &mut next);
    }
    (render, worst, encode)
}

#[test]
fn dashboard_frames_stay_within_the_measured_allocation_ceiling() {
    let (render, worst, encode) = per_frame();
    assert!(
        render <= MAX_RENDER_TOTAL,
        "rendering allocated {render} times over {FRAMES} frames, ceiling {MAX_RENDER_TOTAL}"
    );
    assert!(
        worst <= MAX_RENDER_FRAME,
        "one frame allocated {worst} times, ceiling {MAX_RENDER_FRAME}"
    );
    assert_eq!(encode, 0, "diff and encode allocated in steady state");
}
