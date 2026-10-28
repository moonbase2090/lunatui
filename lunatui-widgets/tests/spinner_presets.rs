// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::time::Duration;

use lunatui_core::{ClusterTable, Grid, Rect, Size, Style, width};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{RenderCx, Spinner, SpinnerPattern, SpinnerPreset, Widget};

fn draw(spinner: Spinner, columns: u16, fill: Option<&str>) -> String {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(columns, 1);
    let mut clusters = ClusterTable::new();
    if let Some(fill) = fill {
        grid.put_str(0, 0, Style::new(), fill, &mut clusters);
    }
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    spinner.render(Rect::new(0, 0, columns, 1), &mut cx);
    grid.row_display(0, &clusters)
}

#[test]
fn the_catalog_lists_the_named_presets() {
    let names: Vec<&str> = SpinnerPreset::ALL
        .iter()
        .map(|preset| preset.name())
        .collect();
    assert_eq!(
        names,
        [
            "dots",
            "line",
            "arc",
            "bounce",
            "pulse",
            "moon",
            "ring",
            "bars",
            "quadrant",
            "half-moon",
        ]
    );
    let mut seen = Vec::new();
    for preset in SpinnerPreset::ALL {
        assert!(preset.interval() > Duration::ZERO);
        assert_ne!(preset.frames(), &[] as &[&str]);
        assert!(!seen.contains(&preset.name()));
        seen.push(preset.name());
        for frame in preset.frames() {
            assert_ne!(*frame, "");
            assert!(width(frame) <= 8);
        }
    }
}

#[test]
fn built_in_frames_and_intervals_are_fixed() {
    assert_eq!(
        SpinnerPreset::DOTS.frames(),
        &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
    );
    assert_eq!(SpinnerPreset::DOTS.interval(), Duration::from_millis(80));
    assert_eq!(SpinnerPreset::LINE.frames(), &["|", "/", "-", "\\"]);
    assert_eq!(SpinnerPreset::LINE.interval(), Duration::from_millis(120));
    assert_eq!(SpinnerPreset::ARC.frames(), &["◜", "◝", "◞", "◟"]);
    assert_eq!(SpinnerPreset::ARC.interval(), Duration::from_millis(100));
    assert_eq!(
        SpinnerPreset::BOUNCE.frames(),
        &[
            "●", " ●", "  ●", "   ●", "    ●", "     ●", "    ●", "   ●", "  ●", " ●",
        ]
    );
    assert_eq!(SpinnerPreset::BOUNCE.interval(), Duration::from_millis(80));
    assert_eq!(
        SpinnerPreset::PULSE.frames(),
        &["∙", "•", "●", "◉", "●", "•"]
    );
    assert_eq!(SpinnerPreset::PULSE.interval(), Duration::from_millis(160));
    assert_eq!(
        SpinnerPreset::MOON.frames(),
        &["○", "◔", "◑", "◕", "●", "◕", "◑", "◔"]
    );
    assert_eq!(SpinnerPreset::MOON.interval(), Duration::from_millis(200));
    assert_eq!(
        SpinnerPreset::RING.frames(),
        &["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"]
    );
    assert_eq!(SpinnerPreset::RING.interval(), Duration::from_millis(90));
    assert_eq!(
        SpinnerPreset::BARS.frames(),
        &["▁▃▅", "▃▅▇", "▅▇▅", "▇▅▃", "▅▃▁", "▃▁▃"]
    );
    assert_eq!(SpinnerPreset::BARS.interval(), Duration::from_millis(110));
    assert_eq!(SpinnerPreset::QUADRANT.frames(), &["▖", "▗", "▝", "▘"]);
    assert_eq!(
        SpinnerPreset::QUADRANT.interval(),
        Duration::from_millis(140)
    );
    assert_eq!(SpinnerPreset::HALF_MOON.frames(), &["◐", "◓", "◑", "◒"]);
    assert_eq!(
        SpinnerPreset::HALF_MOON.interval(),
        Duration::from_millis(180)
    );
}

#[test]
fn single_column_presets_stay_one_cell_wide() {
    for preset in [
        SpinnerPreset::DOTS,
        SpinnerPreset::LINE,
        SpinnerPreset::ARC,
        SpinnerPreset::PULSE,
        SpinnerPreset::MOON,
        SpinnerPreset::RING,
        SpinnerPreset::QUADRANT,
        SpinnerPreset::HALF_MOON,
    ] {
        for frame in preset.frames() {
            assert_eq!(width(frame), 1, "{frame}");
        }
        assert_eq!(
            Spinner::new(0)
                .preset(preset)
                .measure(Size::new(8, 1))
                .width,
            1
        );
    }
    for frame in SpinnerPreset::BARS.frames() {
        assert_eq!(width(frame), 3);
    }
    assert_eq!(
        Spinner::new(0)
            .preset(SpinnerPreset::BOUNCE)
            .measure(Size::new(20, 1))
            .width,
        6
    );
}

#[test]
fn a_shorter_preset_frame_clears_the_previous_glyph() {
    let preset = SpinnerPreset::new("custom", &["abcd", "x"], 40);
    assert_eq!(
        draw(Spinner::new(0).preset(preset), 4, Some("ZZZZ")),
        "abcd"
    );
    assert_eq!(
        draw(Spinner::new(1).preset(preset), 4, Some("ZZZZ")),
        "x   "
    );
    assert_eq!(
        draw(
            Spinner::new(0).preset(SpinnerPreset::BOUNCE),
            6,
            Some("ZZZZZZ")
        ),
        "●     "
    );
    assert_eq!(
        draw(
            Spinner::new(5).preset(SpinnerPreset::BOUNCE),
            6,
            Some("ZZZZZZ")
        ),
        "     ●"
    );
    assert_eq!(
        draw(Spinner::new(2).preset(SpinnerPreset::LINE), 1, None),
        "-"
    );
    assert_eq!(
        draw(Spinner::new(4).preset(SpinnerPreset::LINE), 1, None),
        "|"
    );
}

#[test]
fn selecting_a_pattern_drops_the_preset() {
    let spinner = Spinner::new(0)
        .preset(SpinnerPreset::LINE)
        .pattern(SpinnerPattern::Arc);
    assert_eq!(draw(spinner, 3, Some("ZZZ")), "◜ZZ");
    let spinner = Spinner::new(0)
        .pattern(SpinnerPattern::Arc)
        .preset(SpinnerPreset::LINE);
    assert_eq!(draw(spinner, 1, Some("Z")), "|");
}

#[test]
fn the_default_spinner_still_draws_one_braille_cell() {
    assert_eq!(draw(Spinner::new(0), 3, Some("ZZZ")), "⠋ZZ");
    assert_eq!(Spinner::new(3).measure(Size::new(4, 1)).width, 1);
}

#[test]
fn an_empty_preset_draws_nothing() {
    let preset = SpinnerPreset::new("empty", &[], 10);
    assert_eq!(draw(Spinner::new(0).preset(preset), 3, Some("ZZZ")), "ZZZ");
    assert_eq!(
        Spinner::new(0)
            .preset(preset)
            .measure(Size::new(4, 1))
            .width,
        0
    );
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(2, 1);
    let mut clusters = ClusterTable::new();
    grid.put_str(0, 0, Style::new(), "ZZ", &mut clusters);
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Spinner::new(0)
        .preset(SpinnerPreset::BOUNCE)
        .render(Rect::new(0, 0, 0, 1), &mut cx);
    assert_eq!(grid.row_display(0, &clusters), "ZZ");
}
