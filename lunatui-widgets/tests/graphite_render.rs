//! Contract tests for the feature-gated Graphite render adapters.

use lunatui_core::Rect as CellRect;
use lunatui_theme::Token;
use lunatui_widgets::graphite::{CellSize, CellTextMetrics, Face, Rect, TextMetrics};
use std::num::NonZeroU16;

fn cell_size(width: u16, height: u16) -> CellSize {
    CellSize::new(
        NonZeroU16::new(width).unwrap(),
        NonZeroU16::new(height).unwrap(),
    )
}

#[test]
fn pixel_rect_rounds_outward_and_offsets_from_area() {
    let actual = Rect {
        x: 1,
        y: 1,
        w: 8,
        h: 15,
    }
    .to_cell_rect(CellRect::new(3, 2, 4, 3), cell_size(8, 16));
    assert_eq!(actual, Some(CellRect::new(3, 2, 2, 1)));
}

#[test]
fn pixel_rect_clips_to_area() {
    let actual = Rect {
        x: 8,
        y: 0,
        w: 20,
        h: 16,
    }
    .to_cell_rect(CellRect::new(5, 4, 2, 1), cell_size(8, 16));
    assert_eq!(actual, Some(CellRect::new(6, 4, 1, 1)));
}

#[test]
fn empty_outside_and_overflow_rectangles_are_ignored() {
    let area = CellRect::new(0, 0, 2, 2);
    let size = cell_size(8, 16);
    assert_eq!(
        Rect {
            x: 0,
            y: 0,
            w: 0,
            h: 1
        }
        .to_cell_rect(area, size),
        None
    );
    assert_eq!(
        Rect {
            x: 40,
            y: 0,
            w: 8,
            h: 16
        }
        .to_cell_rect(area, size),
        None
    );
    assert_eq!(
        Rect {
            x: usize::MAX,
            y: 0,
            w: 2,
            h: 1
        }
        .to_cell_rect(area, size),
        None
    );
}

#[test]
fn text_metrics_use_terminal_display_width() {
    let metrics = CellTextMetrics::new(cell_size(8, 16));
    assert_eq!(metrics.width(Face::Regular, 12.0, "abc"), 24.0);
    assert_eq!(metrics.width(Face::Mono, 12.0, "界"), 16.0);
    assert_eq!(metrics.width(Face::SemiBold, 12.0, "e\u{301}"), 8.0);
    assert_eq!(metrics.width(Face::Regular, 12.0, "🇺🇸"), 16.0);
}

#[test]
fn graphite_render_feature_reuses_graphite_token_set() {
    assert_eq!(Token::COUNT, 20);
}
