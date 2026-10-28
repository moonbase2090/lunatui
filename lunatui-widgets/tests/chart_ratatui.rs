// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Axis, Chart, Dataset, GraphType, Marker, RenderCx, Widget};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect as RatRect;
use ratatui::symbols::Marker as RatMarker;
use ratatui::widgets::{
    Axis as RatAxis, Chart as RatChart, Dataset as RatDataset, GraphType as RatGraphType,
    Widget as RatWidget,
};

const LABELS: [&str; 3] = ["0", "5", "9"];

type Series = Vec<(f64, f64)>;

fn ours(
    sets: &[(Series, Marker, GraphType)],
    bounds: ([f64; 2], [f64; 2]),
    size: (u16, u16),
) -> Vec<String> {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(size.0, size.1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    let datasets: Vec<Dataset<'_>> = sets
        .iter()
        .map(|(data, marker, graph)| Dataset::new(data).marker(*marker).graph_type(*graph))
        .collect();
    Chart::new(&datasets)
        .x_axis(Axis::new().bounds(bounds.0).labels(&LABELS))
        .y_axis(Axis::new().bounds(bounds.1).labels(&LABELS))
        .render(Rect::new(0, 0, size.0, size.1), &mut cx);
    (0..size.1 - 1)
        .map(|row| grid.row_display(row, &clusters))
        .collect()
}

fn reference(
    sets: &[(Series, Marker, GraphType)],
    bounds: ([f64; 2], [f64; 2]),
    size: (u16, u16),
) -> Vec<String> {
    let marker = |marker| match marker {
        Marker::Dot => RatMarker::Dot,
        Marker::Block => RatMarker::Block,
        Marker::HalfBlock => RatMarker::HalfBlock,
        _ => RatMarker::Braille,
    };
    let graph = |graph| match graph {
        GraphType::Line => RatGraphType::Line,
        GraphType::Bar => RatGraphType::Bar,
        _ => RatGraphType::Scatter,
    };
    let datasets: Vec<RatDataset<'_>> = sets
        .iter()
        .map(|(data, m, g)| {
            RatDataset::default()
                .marker(marker(*m))
                .graph_type(graph(*g))
                .data(data)
        })
        .collect();
    let chart = RatChart::new(datasets)
        .x_axis(RatAxis::default().bounds(bounds.0).labels(LABELS))
        .y_axis(RatAxis::default().bounds(bounds.1).labels(LABELS));
    let area = RatRect::new(0, 0, size.0, size.1);
    let mut buffer = Buffer::empty(area);
    RatWidget::render(chart, area, &mut buffer);
    (0..size.1 - 1)
        .map(|y| {
            (0..size.0)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect()
        })
        .collect()
}

fn shapes() -> Vec<(&'static str, Series)> {
    vec![
        ("shallow rising", vec![(1.0, 1.0), (18.0, 4.0)]),
        ("shallow falling", vec![(1.0, 8.0), (18.0, 5.0)]),
        ("steep rising", vec![(3.0, 0.5), (6.0, 9.5)]),
        ("steep falling", vec![(15.0, 9.5), (12.0, 0.5)]),
        ("vertical", vec![(7.0, 1.0), (7.0, 9.0)]),
        ("horizontal", vec![(2.0, 4.0), (17.0, 4.0)]),
        ("right to left", vec![(18.0, 2.0), (2.0, 7.0)]),
        ("top to bottom", vec![(5.0, 9.0), (9.0, 1.0)]),
        ("single point", vec![(10.0, 5.0)]),
        (
            "zigzag",
            vec![(0.0, 0.0), (4.0, 9.0), (8.0, 1.0), (12.0, 8.0), (20.0, 2.0)],
        ),
        ("out the left", vec![(-8.0, 3.0), (10.0, 7.0)]),
        ("out the right", vec![(10.0, 2.0), (30.0, 8.0)]),
        ("out the top", vec![(4.0, 5.0), (8.0, 16.0)]),
        ("out the bottom", vec![(6.0, 4.0), (14.0, -9.0)]),
        ("across the corner", vec![(-5.0, 7.0), (7.0, 15.0)]),
        ("through", vec![(-10.0, -10.0), (30.0, 20.0)]),
        ("fully outside", vec![(30.0, 30.0), (40.0, 35.0)]),
        ("on the edge", vec![(0.0, 0.0), (20.0, 10.0)]),
        (
            "scatter cloud",
            (0..30)
                .map(|i| (f64::from(i) * 0.67, f64::from((i * 7) % 11)))
                .collect(),
        ),
    ]
}

#[test]
fn plotted_glyphs_match_the_reference_chart() {
    let markers = [
        Marker::Braille,
        Marker::HalfBlock,
        Marker::Dot,
        Marker::Block,
    ];
    let graphs = [GraphType::Line, GraphType::Scatter];
    for size in [(40, 12), (23, 9), (61, 17), (12, 5)] {
        for bounds in [([0.0, 20.0], [0.0, 10.0]), ([-5.0, 25.0], [-2.0, 12.0])] {
            for (name, data) in shapes() {
                for marker in markers {
                    for graph in graphs {
                        let sets = [(data.clone(), marker, graph)];
                        assert_eq!(
                            ours(&sets, bounds, size),
                            reference(&sets, bounds, size),
                            "{name} {marker:?} {graph:?} {bounds:?} {size:?}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn later_datasets_cover_earlier_ones_like_the_reference_chart() {
    let first = vec![(0.0, 0.0), (20.0, 10.0)];
    let second = vec![(0.0, 10.0), (20.0, 0.0)];
    let third: Series = (0..20).map(|i| (f64::from(i), 5.0)).collect();
    let sets = [
        (first, Marker::Braille, GraphType::Line),
        (second, Marker::Braille, GraphType::Line),
        (third, Marker::HalfBlock, GraphType::Scatter),
    ];
    for size in [(40, 12), (25, 7)] {
        let bounds = ([0.0, 20.0], [0.0, 10.0]);
        assert_eq!(ours(&sets, bounds, size), reference(&sets, bounds, size));
    }
}
