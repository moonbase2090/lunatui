// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::hint::black_box;

use lunatui_bench::CountingAllocator;
use lunatui_core::Color;
use lunatui_core::paint::{Opacity, PreparedRamp, RampOutput, blend_rgb};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

#[test]
fn sampling_and_compositing_allocate_nothing_and_return_the_expected_colors()
-> Result<(), Box<dyn std::error::Error>> {
    let ramp = PreparedRamp::<3>::new([0; 3], [255; 3], RampOutput::TrueColor)?;
    let opacity = Opacity::new(64);
    let (sum, stats) = ALLOC.measure(|| {
        let mut sum = 0_u64;
        for _ in 0..100 {
            for phase in black_box([0, 32_768, u16::MAX]) {
                if let Color::Rgb(red, green, blue) = black_box(&ramp).sample(phase) {
                    sum += u64::from(red) + u64::from(green) + u64::from(blue);
                }
                for channel in blend_rgb(black_box([10, 20, 30]), [110, 120, 130], opacity) {
                    sum += u64::from(channel);
                }
            }
        }
        sum
    });
    assert_eq!(sum, 146_700);
    assert_eq!(stats.allocations, 0);
    Ok(())
}
