// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_bench::widget_suite::{CASES, Case, Parity, SIZES, render_both};
use lunatui_core::{ClusterTable, Color, DecodedGlyph, Grid};
use ratatui::buffer::Buffer;
use ratatui::style::{Color as RatColor, Modifier};

fn lunatui_color(color: RatColor) -> Color {
    match color {
        RatColor::Reset => Color::Default,
        RatColor::Black => Color::Indexed(0),
        RatColor::Red => Color::Indexed(1),
        RatColor::Green => Color::Indexed(2),
        RatColor::Yellow => Color::Indexed(3),
        RatColor::Blue => Color::Indexed(4),
        RatColor::Magenta => Color::Indexed(5),
        RatColor::Cyan => Color::Indexed(6),
        RatColor::Gray => Color::Indexed(7),
        RatColor::DarkGray => Color::Indexed(8),
        RatColor::LightRed => Color::Indexed(9),
        RatColor::LightGreen => Color::Indexed(10),
        RatColor::LightYellow => Color::Indexed(11),
        RatColor::LightBlue => Color::Indexed(12),
        RatColor::LightMagenta => Color::Indexed(13),
        RatColor::LightCyan => Color::Indexed(14),
        RatColor::White => Color::Indexed(15),
        RatColor::Indexed(index) => Color::Indexed(index),
        RatColor::Rgb(red, green, blue) => Color::Rgb(red, green, blue),
    }
}

fn differences(
    grid: &Grid,
    clusters: &ClusterTable,
    buffer: &Buffer,
    width: u16,
    height: u16,
) -> Vec<String> {
    let mut out = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let want = &buffer[(x, y)];
            let Some(cell) = grid.get(x, y) else {
                out.push(format!("({x},{y}) missing cell"));
                continue;
            };
            let symbol = match cell.glyph().decode() {
                DecodedGlyph::Inline(scalar) => scalar.to_string(),
                DecodedGlyph::Cluster(id) => clusters
                    .cluster_text(id)
                    .map(str::to_owned)
                    .unwrap_or_default(),
                DecodedGlyph::Continuation => String::new(),
            };
            let style = cell.style();
            let got = (
                symbol,
                style.fg,
                style.bg,
                style.attrs.bold(),
                style.attrs.dim(),
            );
            let expected = (
                want.symbol().to_owned(),
                lunatui_color(want.fg),
                lunatui_color(want.bg),
                want.modifier.contains(Modifier::BOLD),
                want.modifier.contains(Modifier::DIM),
            );
            if got != expected {
                out.push(format!("({x},{y}) lunatui={got:?} ratatui={expected:?}"));
            }
        }
    }
    out
}

fn check(case: &'static Case, width: u16, height: u16, tick: u32) -> Result<(), String> {
    let (grid, clusters, buffer) = render_both(case, width, height, tick);
    let diffs = differences(&grid, &clusters, &buffer, width, height);
    if diffs.is_empty() {
        return Ok(());
    }
    let shown: Vec<_> = diffs.iter().take(3).collect();
    Err(format!(
        "{} {width}x{height} tick {tick}: {} differing cells, first: {shown:#?}",
        case.name,
        diffs.len()
    ))
}

#[test]
fn exact_cases_match_ratatui_cell_for_cell() {
    let mut failures = Vec::new();
    for case in CASES {
        if case.parity != Parity::Exact {
            continue;
        }
        for (width, height) in SIZES {
            for tick in [0, 1, 2, 3, 7, 13, 49, 99] {
                if let Err(message) = check(case, width, height, tick) {
                    failures.push(message);
                    break;
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn documented_cases_state_a_reason() {
    for case in CASES {
        if let Parity::Documented(reason) = case.parity {
            assert!(!reason.is_empty(), "{} has an empty reason", case.name);
        }
    }
}

#[test]
fn checkbox_matches_tui_checkbox_across_all_scripted_states_and_themes() -> Result<(), String> {
    use lunatui_bench::widget_suite::checkbox;
    use lunatui_core::Rect;
    use lunatui_theme::{Theme, Variant};
    use ratatui::layout::Rect as RatRect;

    for variant in [Variant::Dark, Variant::Light] {
        let theme = Theme::builtin(variant);
        for (width, height) in [(0, 0), (7, 3), (80, 24), (200, 60)] {
            for tick in [0, 12, 24, 36, 48, 60, 108, 120, 168, 180, 228, 239] {
                let mut grid = Grid::new(width, height);
                let mut clusters = ClusterTable::new();
                let mut buffer = Buffer::empty(RatRect::new(0, 0, width, height));
                checkbox::native(
                    &mut grid,
                    &mut clusters,
                    &theme,
                    Rect::new(0, 0, width, height),
                    tick,
                );
                checkbox::baseline_themed(
                    &mut buffer,
                    RatRect::new(0, 0, width, height),
                    &theme,
                    tick,
                );
                let diffs = differences(&grid, &clusters, &buffer, width, height);
                if !diffs.is_empty() {
                    return Err(format!(
                        "checkbox {variant:?} {width}x{height} tick {tick}: {:?}",
                        &diffs[..diffs.len().min(3)]
                    ));
                }
                if (width, height) == (80, 24) {
                    let expected = if tick < 60 {
                        "[ ] Deploys"
                    } else {
                        "[x] Deploys"
                    };
                    assert!(
                        grid.row_display(6, &clusters).contains(expected),
                        "missing actual checkbox at tick {tick}"
                    );
                    let group = if tick < 60 {
                        "[ ] All events"
                    } else if tick < 180 {
                        "[-] All events"
                    } else {
                        "[x] All events"
                    };
                    assert!(grid.row_display(16, &clusters).contains(group));
                    assert!(
                        grid.row_display(14, &clusters)
                            .contains("[x] Weekly summary")
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn widget_list_matches_tui_widget_list_across_every_selection_and_theme() -> Result<(), String> {
    use lunatui_bench::widget_suite::widget_list;
    use lunatui_core::Rect;
    use lunatui_theme::{Theme, Variant};
    use ratatui::layout::Rect as RatRect;

    for variant in [Variant::Dark, Variant::Light] {
        let theme = Theme::builtin(variant);
        for (width, height) in [(0, 0), (7, 3), (13, 11), (80, 24), (200, 60)] {
            for tick in (0..240).step_by(10) {
                let mut grid = Grid::new(width, height);
                let mut clusters = ClusterTable::new();
                let mut buffer = Buffer::empty(RatRect::new(0, 0, width, height));
                widget_list::native(
                    &mut grid,
                    &mut clusters,
                    &theme,
                    Rect::new(0, 0, width, height),
                    tick,
                );
                widget_list::baseline_themed(
                    &mut buffer,
                    RatRect::new(0, 0, width, height),
                    &theme,
                    tick,
                );
                let diffs = differences(&grid, &clusters, &buffer, width, height);
                if !diffs.is_empty() {
                    return Err(format!(
                        "widget_list {variant:?} {width}x{height} tick {tick}: {:?}",
                        &diffs[..diffs.len().min(3)]
                    ));
                }
                if (width, height) == (80, 24) && variant == Variant::Dark {
                    let screen: String = (0..height)
                        .map(|y| grid.row_display(y, &clusters))
                        .collect::<Vec<_>>()
                        .join("\n");
                    let expect = |needle: &str, shown: bool| {
                        assert_eq!(
                            screen.contains(needle),
                            shown,
                            "tick {tick}: {needle:?} in\n{screen}"
                        );
                    };
                    match tick {
                        0 => {
                            expect("Today", true);
                            expect("Deploy blocked", true);
                        }
                        40 => {
                            expect("Deploy blocked", true);
                            expect("Today", true);
                        }
                        60 => {
                            expect("Cache warm-up", true);
                            expect("Today", false);
                        }
                        200 => {
                            expect("Cache warm-up", true);
                            expect("Today", false);
                            expect("Deploy blocked", true);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}

#[test]
fn heatmap_matches_its_independent_buffer_reference() -> Result<(), String> {
    let case = CASES
        .iter()
        .find(|case| case.name == "heatmap")
        .ok_or("missing heatmap case")?;
    for (width, height) in SIZES {
        for tick in [0, 1, 30, 75, 150, 299] {
            check(case, width, height, tick)?;
        }
    }
    Ok(())
}

#[test]
fn timeline_matches_its_independent_buffer_reference() -> Result<(), String> {
    let case = CASES
        .iter()
        .find(|case| case.name == "timeline")
        .ok_or("missing timeline case")?;
    for (width, height) in SIZES {
        for tick in [0, 1, 20, 47, 100, 149, 150, 299] {
            check(case, width, height, tick)?;
        }
    }
    Ok(())
}

#[test]
fn split_keyboard_case_moves_the_divider_in_whole_cells() -> Result<(), String> {
    let case = CASES
        .iter()
        .find(|case| case.name == "split")
        .ok_or("missing split case")?;
    for (width, height, columns) in [(80, 24, [33, 34, 35]), (200, 60, [84, 85, 86])] {
        let (grid, _, _) = render_both(case, width, height, 13);
        for (column, glyph) in columns.into_iter().zip(['╮', '│', '╭']) {
            assert_eq!(
                grid.get(column, 0).map(|cell| cell.glyph().decode()),
                Some(DecodedGlyph::Inline(glyph)),
                "{width}x{height} column {column}"
            );
        }
    }
    Ok(())
}

#[test]
fn every_case_has_a_unique_name() {
    let mut names: Vec<_> = CASES.iter().map(|case| case.name).collect();
    names.sort_unstable();
    let before = names.len();
    names.dedup();
    assert_eq!(before, names.len());
}

#[test]
fn terminal_parity_covers_the_colored_output_and_an_observable_frame_change() -> Result<(), String>
{
    let case = CASES
        .iter()
        .find(|case| case.name == "terminal")
        .ok_or("missing terminal case")?;
    for (width, height) in SIZES {
        for tick in [0, 1, 49, 99] {
            check(case, width, height, tick)?;
        }
        let (grid, clusters, _) = render_both(case, width, height, 49);
        assert_eq!(
            grid.row_display(0, &clusters).trim_end(),
            "orbit % cargo test"
        );
        assert_eq!(
            grid.row_display(7, &clusters).trim_end(),
            "finished in 0.49s"
        );
        let cell = grid.get(0, 1).ok_or("missing compile label")?;
        assert_eq!(cell.style().fg, Color::Indexed(4));
        assert!(cell.style().attrs.bold());
    }
    Ok(())
}

#[test]
fn mermaid_cells_match_the_independent_fixed_diagram_reference() -> Result<(), String> {
    let case = CASES
        .iter()
        .find(|case| case.name == "mermaid")
        .ok_or("missing Mermaid case")?;
    for (width, height) in SIZES {
        for tick in 0..2 {
            check(case, width, height, tick)?;
            let (grid, clusters, _) = render_both(case, width, height, tick);
            assert_eq!(
                grid.row_display(0, &clusters).trim_end(),
                if tick == 0 {
                    "┌───┐"
                } else {
                    "┌───┐  ┌───┐"
                }
            );
            assert!(
                grid.display_text(&clusters)
                    .contains(if tick == 0 { "▼" } else { "▶" })
            );
        }
    }
    Ok(())
}

#[test]
fn command_palette_matches_an_independent_reference_in_empty_and_fuzzy_states() -> Result<(), String>
{
    let case = CASES
        .iter()
        .find(|case| case.name == "command-palette")
        .ok_or("missing palette")?;
    for (width, height) in SIZES {
        for tick in 0..3 {
            check(case, width, height, tick)?;
            let (grid, clusters, _) = render_both(case, width, height, tick);
            let text = grid.display_text(&clusters);
            assert!(text.contains(match tick {
                0 => "Close tab",
                1 => "Open file",
                _ => "Save file",
            }));
            if tick != 0 {
                assert!(!text.contains("Close tab"));
            }
        }
    }
    Ok(())
}

#[test]
fn effects_match_the_hand_written_fade_across_the_whole_timeline() -> Result<(), String> {
    use lunatui_bench::widget_suite::effects;
    use lunatui_core::Rect;
    use lunatui_theme::{Theme, Variant};
    use ratatui::layout::Rect as RatRect;

    let case = CASES
        .iter()
        .find(|case| case.name == "effects")
        .ok_or("missing effects case")?;
    for (width, height) in SIZES {
        for tick in (0..240).step_by(7).chain([59, 60, 179, 180, 239]) {
            check(case, width, height, tick)?;
        }
    }
    let light = Theme::builtin(Variant::Light);
    for (width, height) in SIZES {
        for tick in [0, 30, 100, 200] {
            let mut grid = Grid::new(width, height);
            let mut clusters = ClusterTable::new();
            let mut buffer = Buffer::empty(RatRect::new(0, 0, width, height));
            effects::native(
                &mut grid,
                &mut clusters,
                &light,
                Rect::new(0, 0, width, height),
                tick,
            );
            effects::baseline_themed(&mut buffer, RatRect::new(0, 0, width, height), &light, tick);
            let diffs = differences(&grid, &clusters, &buffer, width, height);
            if !diffs.is_empty() {
                return Err(format!(
                    "effects light {width}x{height} tick {tick}: {:?}",
                    &diffs[..diffs.len().min(3)]
                ));
            }
        }
    }
    let (width, height) = SIZES[0];
    let background = |tick: u32| {
        let (grid, _, _) = render_both(case, width, height, tick);
        (0..width)
            .map(|x| grid.get(x, 0).map(|cell| cell.style().bg))
            .collect::<Vec<_>>()
    };
    let text = |tick: u32| {
        let (grid, clusters, _) = render_both(case, width, height, tick);
        grid.display_text(&clusters)
    };
    assert_eq!(background(0), background(239));
    let held = text(100);
    assert!(held.contains("integration  3 failed"));
    assert_eq!(text(30), held);
    let (faded, _, _) = render_both(case, width, height, 30);
    let (steady, _, _) = render_both(case, width, height, 100);
    let moved = (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .filter(|&(x, y)| faded.get(x, y) != steady.get(x, y))
        .count();
    assert!(moved > 500, "only {moved} cells changed at tick 30");
    let (a, _, _) = render_both(case, width, height, 60);
    let (b, _, _) = render_both(case, width, height, 179);
    assert_eq!(a.cell_bits(40, 12), b.cell_bits(40, 12));
    Ok(())
}

#[test]
fn node_graph_matches_tui_nodes_across_selection_themes_and_offsets() -> Result<(), String> {
    use lunatui_bench::widget_suite::node_graph;
    use lunatui_core::Rect;
    use lunatui_theme::{Theme, Variant};
    use ratatui::layout::Rect as RatRect;
    for variant in [Variant::Dark, Variant::Light] {
        let theme = Theme::builtin(variant);
        for (width, height) in [(0_u16, 0_u16), (7, 3), (80, 24), (200, 60)] {
            for inset in [0, 2] {
                let area = Rect::new(
                    inset,
                    inset,
                    width.saturating_sub(inset * 2),
                    height.saturating_sub(inset * 2),
                );
                for tick in [0, 1, 29, 59, 60, 89, 119, 120, 149, 179, 180, 209, 239] {
                    let mut grid = Grid::new(width, height);
                    let mut clusters = ClusterTable::new();
                    let mut buffer = Buffer::empty(RatRect::new(0, 0, width, height));
                    node_graph::native(&mut grid, &mut clusters, &theme, area, tick);
                    node_graph::baseline_themed(
                        &mut buffer,
                        RatRect::new(area.x, area.y, area.width, area.height),
                        &theme,
                        tick,
                    );
                    let diffs = differences(&grid, &clusters, &buffer, width, height);
                    if !diffs.is_empty() {
                        return Err(format!(
                            "node graph {variant:?} {width}x{height} inset {inset} tick {tick}: {:?}",
                            &diffs[..diffs.len().min(3)]
                        ));
                    }
                    if width >= 80 {
                        let text: Vec<_> = (0..height)
                            .map(|y| grid.row_display(y, &clusters))
                            .collect();
                        assert!(text.iter().any(|line| line.contains("Repository")));
                        assert!(text.iter().any(|line| line.contains("Package")));
                        assert_eq!(
                            text.iter().any(|line| line.contains("published")),
                            tick >= 180
                        );
                        assert!(text.iter().any(|line| line.contains("├─────┤")));
                    }
                }
            }
        }
    }
    Ok(())
}

#[test]
fn node_graph_captions_erase_previous_steps_in_reused_buffers() {
    use lunatui_bench::widget_suite::node_graph;
    use lunatui_core::Rect;
    use lunatui_theme::{Theme, Variant};
    use ratatui::layout::Rect as RatRect;

    for variant in [Variant::Dark, Variant::Light] {
        let theme = Theme::builtin(variant);
        for (width, height) in [(80, 24), (200, 60)] {
            let area = Rect::new(0, 0, width, height);
            let rat_area = RatRect::new(0, 0, width, height);
            let mut grid = Grid::new(width, height);
            let mut clusters = ClusterTable::new();
            let mut buffer = Buffer::empty(rat_area);
            for (tick, expected) in [
                (0, "01  Repository synced"),
                (59, "01  Repository synced"),
                (60, "02  Checks passed"),
                (119, "02  Checks passed"),
                (120, "03  Package prepared"),
                (179, "03  Package prepared"),
                (180, "04  Release published"),
                (239, "04  Release published"),
                (240, "01  Repository synced"),
                (300, "02  Checks passed"),
            ] {
                node_graph::native(&mut grid, &mut clusters, &theme, area, tick);
                node_graph::baseline_themed(&mut buffer, rat_area, &theme, tick);
                assert_eq!(
                    grid.row_display(height - 3, &clusters).trim(),
                    expected,
                    "native {variant:?} {width}x{height} tick {tick}"
                );
                let baseline: String = (0..width)
                    .map(|x| buffer[(x, height - 3)].symbol())
                    .collect();
                assert_eq!(
                    baseline.trim(),
                    expected,
                    "baseline {variant:?} {width}x{height} tick {tick}"
                );
            }
        }
    }
}
