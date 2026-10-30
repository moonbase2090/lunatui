// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_bench::CountingAllocator;
use lunatui_core::Rect;
use lunatui_term::{PixelImage, PixelLayer, PixelMode};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

const SIDE: u32 = 24;

fn pixels(seed: u8) -> Vec<u8> {
    (0..SIDE * SIDE * 4)
        .map(|index| u8::try_from(index % 251).unwrap_or(0).wrapping_add(seed))
        .collect()
}

fn run_frame<'a>(layer: &'a mut PixelLayer, images: &[PixelImage<'_>; 3], step: u64) -> &'a [u8] {
    let moving = u16::try_from(step % 8).unwrap_or(0);
    layer.begin_frame();
    layer.image(1, 1, &images[0], Rect::new(moving, 1, 6, 3), 0);
    layer.image(2, 1, &images[1], Rect::new(20, 4, 8, 4), -1);
    layer.image(3, step / 5, &images[2], Rect::new(2, 10, 6, 3), 1);
    PixelLayer::finish_frame(layer)
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

#[test]
fn steady_state_frames_allocate_nothing_and_send_only_what_changed() -> Result<(), String> {
    let (a, b, c) = (pixels(0), pixels(1), pixels(2));
    let images = [
        PixelImage::new(SIDE, SIDE, &a).ok_or("image a")?,
        PixelImage::new(SIDE, SIDE, &b).ok_or("image b")?,
        PixelImage::new(SIDE, SIDE, &c).ok_or("image c")?,
    ];
    let mut layer = PixelLayer::new(PixelMode::Kitty);
    for step in 0..40 {
        run_frame(&mut layer, &images, step);
    }
    let mut allocations = 0;
    for step in 40..100_u64 {
        let target = &mut layer;
        let (output, stats) = ALLOC.measure(move || run_frame(target, &images, step));
        allocations += stats.allocations;
        if !contains(output, b"a=p,i=1,") {
            return Err(format!("step {step}: the moving image was not placed"));
        }
        if contains(output, b"i=2,") {
            return Err(format!("step {step}: the static image was sent again"));
        }
        if contains(output, b"a=t,i=1,") {
            return Err(format!(
                "step {step}: the moving image was transmitted again"
            ));
        }
        let bumped = contains(output, b"a=t,i=");
        if bumped != (step % 5 == 0) {
            return Err(format!("step {step}: transmit of image 3 was {bumped}"));
        }
    }
    if allocations == 0 {
        Ok(())
    } else {
        Err(format!(
            "the pixel layer allocated {allocations} times in steady state"
        ))
    }
}
