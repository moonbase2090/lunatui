// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::paint::{Opacity, PreparedRamp, RampOutput, blend_rgb};
use lunatui_core::{Color, oklab};

#[test]
fn red_matches_the_published_oklab_reference() {
    let coordinates = oklab::from_rgb([255, 0, 0]).components();
    for (got, expected) in coordinates
        .into_iter()
        .zip([0.627_955_36, 0.224_863_06, 0.125_846_30])
    {
        assert!((got - expected).abs() < 0.000_001);
    }
    assert_eq!(oklab::to_rgb(oklab::from_rgb([255, 0, 0])), [255, 0, 0]);
}

#[test]
fn perceptual_midpoint_and_progress_boundaries_are_literal() {
    assert_eq!(oklab::lerp_rgb([0; 3], [255; 3], 0.5), [99; 3]);
    assert_eq!(
        oklab::lerp_rgb([255, 0, 0], [0, 0, 255], 0.5),
        [140, 83, 162]
    );
    for phase in [f32::NAN, f32::NEG_INFINITY, -1.0, 0.0] {
        assert_eq!(oklab::lerp_rgb([1, 2, 3], [7, 8, 9], phase), [1, 2, 3]);
    }
    for phase in [1.0, 2.0, f32::INFINITY] {
        assert_eq!(oklab::lerp_rgb([1, 2, 3], [7, 8, 9], phase), [7, 8, 9]);
    }
}

#[test]
fn byte_channels_round_trip_without_losing_values() {
    for byte in 0..=255 {
        let sample = [byte, 255 - byte, byte / 2];
        assert_eq!(oklab::to_rgb(oklab::from_rgb(sample)), sample);
    }
}

#[test]
fn source_over_preserves_endpoints_and_graphite_integer_mixing() {
    assert_eq!(
        blend_rgb([10, 20, 30], [110, 120, 130], Opacity::new(64)),
        [35, 45, 55]
    );
    assert_eq!(
        blend_rgb([10, 20, 30], [110, 120, 130], Opacity::TRANSPARENT),
        [10, 20, 30]
    );
    assert_eq!(
        blend_rgb([10, 20, 30], [110, 120, 130], Opacity::OPAQUE),
        [110, 120, 130]
    );
    assert_eq!(
        blend_rgb([0; 3], [255; 3], Opacity::new(u16::MAX)),
        [255; 3]
    );
    assert_eq!(blend_rgb([0; 3], [255; 3], Opacity::new(128)), [127; 3]);
}

#[test]
fn prepared_ramp_samples_endpoints_midpoint_and_clamped_columns()
-> Result<(), Box<dyn std::error::Error>> {
    let ramp = PreparedRamp::<3>::new([0; 3], [255; 3], RampOutput::TrueColor)?;
    assert_eq!(
        ramp.colors(),
        &[
            Color::Rgb(0, 0, 0),
            Color::Rgb(99, 99, 99),
            Color::Rgb(255, 255, 255)
        ]
    );
    assert_eq!(ramp.sample(0), Color::Rgb(0, 0, 0));
    assert_eq!(ramp.sample(32_768), Color::Rgb(99, 99, 99));
    assert_eq!(ramp.sample(u16::MAX), Color::Rgb(255, 255, 255));
    assert_eq!(ramp.color_at(usize::MAX), Color::Rgb(255, 255, 255));
    Ok(())
}

#[test]
fn reduced_color_ramps_snap_to_the_resolved_solid_token() -> Result<(), Box<dyn std::error::Error>>
{
    for color in [Color::Indexed(4), Color::Indexed(75), Color::Token(3)] {
        let ramp = PreparedRamp::<8>::new([10, 20, 30], [200, 210, 220], RampOutput::Solid(color))?;
        assert_eq!(ramp.colors(), &[color; 8]);
    }
    assert!(PreparedRamp::<0>::new([0; 3], [255; 3], RampOutput::TrueColor).is_err());
    assert!(PreparedRamp::<1>::new([0; 3], [255; 3], RampOutput::TrueColor).is_err());
    Ok(())
}
