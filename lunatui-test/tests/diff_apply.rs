// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Style, UnderlineStyle};
use lunatui_term::ColorMode;
use lunatui_test::{diff_matches, fuzz_bytes, fuzz_campaign};
use proptest::prelude::*;

const GLYPHS: [char; 6] = ['a', 'b', ' ', 'Z', '日', '中'];

fn style_strategy() -> impl Strategy<Value = Style> {
    (any::<u8>(), any::<u8>(), any::<u8>(), any::<bool>()).prop_map(|(fg, bg, attr, rgb)| {
        let mut attrs = Attrs::new();
        if attr & 2 != 0 {
            attrs = attrs.set_dim(true);
        } else if attr & 1 != 0 {
            attrs = attrs.set_bold(true);
        }
        if attr & 4 != 0 {
            attrs = attrs.set_italic(true);
        }
        if attr & 8 != 0 {
            attrs = attrs.set_underline(UnderlineStyle::Single);
        }
        if attr & 16 != 0 {
            attrs = attrs.set_reverse(true);
        }
        let (foreground, background) = if rgb {
            (Color::Rgb(fg, bg, attr), Color::Rgb(bg, attr, fg))
        } else {
            (Color::Indexed(fg), Color::Indexed(bg))
        };
        Style::new().fg(foreground).bg(background).with_attrs(attrs)
    })
}

fn writes() -> impl Strategy<Value = Vec<(u16, u16, String, Style)>> {
    prop::collection::vec(
        (
            0u16..40,
            0u16..20,
            prop::collection::vec(prop::sample::select(&GLYPHS), 0..6),
            style_strategy(),
        )
            .prop_map(|(x, y, chars, style)| (x, y, chars.into_iter().collect(), style)),
        0..6,
    )
}

fn paint(
    width: u16,
    height: u16,
    ops: &[(u16, u16, String, Style)],
    clusters: &mut ClusterTable,
) -> Grid {
    let mut grid = Grid::new(width, height);
    for (x, y, text, style) in ops {
        grid.put_str(x % width, y % height, *style, text, clusters);
    }
    grid
}

proptest! {
    #[test]
    fn old_plus_encoded_diff_matches_new(
        width in 1u16..20,
        height in 1u16..6,
        first in writes(),
        second in writes(),
    ) {
        let mut clusters = ClusterTable::new();
        let previous = paint(width, height, &first, &mut clusters);
        let result = diff_matches(&Grid::new(width, height), &previous, &clusters, ColorMode::TrueColor);
        prop_assert!(result.is_ok(), "paint {}", result.map_or_else(|err| err.to_string(), |()| String::new()));
        let mut next = previous.clone();
        next.clear_dirty();
        for (x, y, text, style) in &second {
            next.put_str(x % width, y % height, *style, text, &mut clusters);
        }
        let mut oracle = lunatui_test::Oracle::new(height, width);
        let blank = Grid::new(width, height);
        let mut encoder = lunatui_term::Encoder::new();
        let paint = encoder.encode_diff(&blank, &previous, &clusters, ColorMode::TrueColor).to_vec();
        oracle.process(&paint);
        let result = lunatui_test::apply_diff(&mut oracle, &previous, &next, &clusters, ColorMode::TrueColor);
        prop_assert!(result.is_ok(), "diff {}", result.map_or_else(|err| err.to_string(), |()| String::new()));
    }
}

#[test]
fn ascii_diff_round_trips_through_the_oracle() {
    let mut clusters = ClusterTable::new();
    let mut previous = Grid::new(4, 1);
    let mut next = Grid::new(4, 1);
    let style = Style::new().fg(Color::Indexed(2)).bg(Color::Rgb(1, 2, 3));
    previous.put_str(0, 0, style, "ab", &mut clusters);
    next.put_str(
        0,
        0,
        style.with_attrs(Attrs::new().set_bold(true)),
        "aZ",
        &mut clusters,
    );
    let result = diff_matches(&previous, &next, &clusters, ColorMode::TrueColor);
    assert_eq!(
        result.map_or_else(|err| err.to_string(), |()| String::new()),
        ""
    );
}

#[test]
fn no_color_encode_matches_text() {
    let mut clusters = ClusterTable::new();
    let previous = Grid::new(3, 1);
    let mut next = Grid::new(3, 1);
    next.put_str(
        0,
        0,
        Style::new().fg(Color::Indexed(1)),
        "Hi",
        &mut clusters,
    );
    let result = diff_matches(&previous, &next, &clusters, ColorMode::NoColor);
    assert_eq!(
        result.map_or_else(|err| err.to_string(), |()| String::new()),
        ""
    );
}

#[test]
fn different_sizes_are_rejected() {
    let clusters = ClusterTable::new();
    let previous = Grid::new(2, 1);
    let next = Grid::new(3, 1);
    let result = diff_matches(&previous, &next, &clusters, ColorMode::TrueColor);
    assert!(result.is_err());
}

#[test]
fn fuzz_bytes_accept_an_empty_input() {
    assert!(fuzz_bytes(&[]).is_ok());
}

#[test]
fn fuzz_bytes_accept_a_short_corpus() {
    let result = fuzz_bytes(&[0, 1, 2, 3, 9, 0, 4, 255, 7]);
    assert_eq!(
        result.map_or_else(|err| err.to_string(), |()| String::new()),
        ""
    );
}

#[test]
fn fuzz_campaign_smoke_is_clean() {
    let result = fuzz_campaign(64);
    assert_eq!(
        result.map_or_else(|err| err.to_string(), |_| String::new()),
        ""
    );
}
