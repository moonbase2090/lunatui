// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Color, Grid, Style};
use lunatui_theme::Profile;

const RAMP: [[u8; 3]; 8] = [
    [255, 0, 0],
    [255, 255, 0],
    [0, 255, 0],
    [0, 255, 255],
    [0, 0, 255],
    [255, 0, 255],
    [255, 128, 0],
    [255, 255, 255],
];

#[test]
fn exact_palette_colors_keep_their_lowest_index() {
    let cases = [
        ([0, 0, 0], 0),
        ([255, 255, 255], 15),
        ([255, 0, 0], 9),
        ([205, 0, 0], 1),
        ([0, 0, 238], 4),
        ([127, 127, 127], 8),
        ([8, 8, 8], 232),
        ([238, 238, 238], 255),
    ];
    for (rgb, index) in cases {
        let color = Color::Rgb(rgb[0], rgb[1], rgb[2]);
        assert_eq!(
            Profile::Ansi256.downsample(color),
            Color::Indexed(index),
            "{rgb:?}"
        );
    }
}

#[test]
fn ansi16_and_mono_use_the_golden_ramp() {
    let ansi16 = [9, 11, 10, 14, 4, 13, 9, 15];
    let mono = [15, 15, 15, 15, 0, 15, 15, 15];
    let ansi256 = [9, 11, 10, 14, 21, 13, 208, 15];
    for (offset, rgb) in RAMP.iter().enumerate() {
        let color = Color::Rgb(rgb[0], rgb[1], rgb[2]);
        assert_eq!(
            Profile::Ansi256.downsample(color),
            Color::Indexed(ansi256[offset]),
            "{rgb:?}"
        );
        assert_eq!(
            Profile::Ansi16.downsample(color),
            Color::Indexed(ansi16[offset]),
            "{rgb:?}"
        );
        assert_eq!(
            Profile::Mono.downsample(color),
            Color::Indexed(mono[offset]),
            "{rgb:?}"
        );
    }
}

#[test]
fn sixteen_color_gradient_snapshot_keeps_the_hue_steps() {
    let mut grid = Grid::new(8, 1);
    let mut clusters = ClusterTable::new();
    for (offset, rgb) in RAMP.iter().enumerate() {
        let color = Profile::Ansi16.downsample(Color::Rgb(rgb[0], rgb[1], rgb[2]));
        let column = u16::try_from(offset).unwrap_or(0);
        grid.put_str(column, 0, Style::new().fg(color), "█", &mut clusters);
    }
    let snap = lunatui_test::styled_snapshot(&grid, &clusters);
    assert!(snap.contains("█"));
    for (offset, index) in [9, 11, 10, 14, 4, 13, 9, 15].iter().enumerate() {
        let column = u16::try_from(offset).unwrap_or(0);
        let line = format!("0:{column}:1 Indexed({index})");
        assert!(snap.contains(&line), "{line} missing from {snap}");
    }
}

#[test]
fn default_and_token_colors_pass_through_every_profile() {
    let colors = [Color::Default, Color::Token(7)];
    for profile in [
        Profile::TrueColor,
        Profile::Ansi256,
        Profile::Ansi16,
        Profile::Mono,
    ] {
        for color in colors {
            assert_eq!(profile.downsample(color), color);
        }
    }
}

#[test]
fn truecolor_keeps_rgb_and_indexed_colors() {
    let rgb = Color::Rgb(12, 34, 56);
    let indexed = Color::Indexed(200);
    assert_eq!(Profile::TrueColor.downsample(rgb), rgb);
    assert_eq!(Profile::TrueColor.downsample(indexed), indexed);
}

#[test]
fn a_high_index_red_becomes_ansi_red_at_16_colors() {
    assert_eq!(
        Profile::Ansi16.downsample(Color::Indexed(196)),
        Color::Indexed(9)
    );
    assert_eq!(
        Profile::Ansi16.downsample(Color::Indexed(4)),
        Color::Indexed(4)
    );
}

#[test]
fn downsampling_twice_matches_once() {
    let color = Color::Rgb(128, 64, 32);
    for profile in [Profile::Ansi256, Profile::Ansi16, Profile::Mono] {
        let once = profile.downsample(color);
        assert_eq!(profile.downsample(once), once);
    }
    assert_eq!(Profile::Ansi256.downsample(color), Color::Indexed(88));
    assert_eq!(Profile::Ansi16.downsample(color), Color::Indexed(1));
    assert_eq!(Profile::Mono.downsample(color), Color::Indexed(0));
}
