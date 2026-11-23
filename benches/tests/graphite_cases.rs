// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use graphite_core::PixelBuffer;
use lunatui_bench::graphite_cases::{
    CELL_HEIGHT, CELL_WIDTH, GRAPHITE_CASES, GraphiteRun, Repaint, Scene, changed_cells,
    graphite_case, render_changed, render_full,
};
use lunatui_bench::graphite_monitor::Density;

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_decode(text: &str) -> Result<Vec<u8>, String> {
    let body = text
        .strip_suffix("==")
        .or_else(|| text.strip_suffix('='))
        .unwrap_or(text);
    let mut out = Vec::with_capacity(body.len() * 3 / 4);
    let (mut bits, mut count) = (0_u32, 0);
    for byte in body.bytes() {
        let value = ALPHABET
            .iter()
            .position(|candidate| *candidate == byte)
            .ok_or_else(|| format!("byte {byte} is not base64"))?;
        bits = bits << 6 | u32::try_from(value).map_err(|error| error.to_string())?;
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push(u8::try_from((bits >> count) & 0xff).map_err(|error| error.to_string())?);
        }
    }
    Ok(out)
}

fn transmitted(stream: &[u8]) -> Result<((usize, usize), Vec<u8>), String> {
    let stream = std::str::from_utf8(stream).map_err(|error| error.to_string())?;
    let mut size = None;
    let mut payload = Vec::new();
    for command in stream.split("\x1b_G").skip(1) {
        let body = command
            .split("\x1b\\")
            .next()
            .ok_or("unterminated command")?;
        let (keys, data) = body.split_once(';').unwrap_or((body, ""));
        if keys.contains("a=p") || keys.contains("a=d") {
            continue;
        }
        if keys.contains("a=t") {
            if size.is_some() {
                return Err("more than one image transmitted".to_owned());
            }
            let key = |name: &str| {
                keys.split(',')
                    .find_map(|pair| pair.strip_prefix(name))
                    .and_then(|value| value.parse::<usize>().ok())
                    .ok_or_else(|| format!("no {name} in {keys}"))
            };
            size = Some((key("s=")?, key("v=")?));
        }
        payload.extend(base64_decode(data)?);
    }
    Ok((size.ok_or("no image transmitted")?, payload))
}

fn crop(full: &PixelBuffer, x: usize, y: usize, width: usize, height: usize) -> Vec<u8> {
    let stride = full.width() * 4;
    (y..y + height)
        .flat_map(|row| &full.as_rgba()[row * stride + x * 4..row * stride + (x + width) * 4])
        .copied()
        .collect()
}

fn changed_pixels(
    cols: u16,
    rows: u16,
    scale: usize,
) -> Result<(usize, usize, usize, usize), String> {
    let cells = changed_cells(cols, rows)?;
    Ok((
        usize::from(cells.x) * CELL_WIDTH * scale,
        usize::from(cells.y) * CELL_HEIGHT * scale,
        usize::from(cells.width) * CELL_WIDTH * scale,
        usize::from(cells.height) * CELL_HEIGHT * scale,
    ))
}

#[test]
fn a_tick_changes_pixels_only_inside_the_changed_rectangle() -> Result<(), String> {
    for (cols, rows, scale) in [(80, 24, 1), (80, 24, 2), (200, 60, 1)] {
        let (x, y, w, h) = changed_pixels(cols, rows, scale)?;
        for tick in [0, 7, 31] {
            let before = render_full(cols, rows, scale, tick)?;
            let after = render_full(cols, rows, scale, tick + 1)?;
            let mut inside = 0;
            for (index, (a, b)) in before
                .as_rgba()
                .chunks(4)
                .zip(after.as_rgba().chunks(4))
                .enumerate()
            {
                if a != b {
                    let (px, py) = (index % before.width(), index / before.width());
                    assert!(
                        (x..x + w).contains(&px) && (y..y + h).contains(&py),
                        "{cols}x{rows}@{scale}x tick {tick}: pixel ({px},{py}) changed outside the rectangle"
                    );
                    inside += 1;
                }
            }
            assert!(
                inside > 100,
                "{cols}x{rows}@{scale}x tick {tick}: only {inside} pixels changed"
            );
        }
    }
    Ok(())
}

#[test]
fn the_changed_rectangle_is_the_full_frame_cropped() -> Result<(), String> {
    for (cols, rows, scale) in [(80, 24, 1), (80, 24, 2), (200, 60, 1)] {
        let (x, y, w, h) = changed_pixels(cols, rows, scale)?;
        for tick in [0, 5, 40] {
            let full = render_full(cols, rows, scale, tick)?;
            let changed = render_changed(cols, rows, scale, tick)?;
            assert_eq!((changed.width(), changed.height()), (w, h));
            assert!(
                changed.as_rgba() == crop(&full, x, y, w, h).as_slice(),
                "{cols}x{rows}@{scale}x tick {tick}: the changed repaint differs from the full frame"
            );
        }
    }
    Ok(())
}

#[test]
fn a_changed_frame_sends_only_the_rectangle_and_a_full_frame_sends_the_window() -> Result<(), String>
{
    let (cols, rows) = (80, 24);
    for scale in [1, 2] {
        let name = format!("graphite-changed-{scale}x");
        let case = graphite_case(&name).ok_or("no changed case")?;
        let mut run = GraphiteRun::new(case, cols, rows)?;
        let mut stream = Vec::new();
        run.frame_into(&mut stream)
            .map_err(|error| error.to_string())?;
        for tick in 1..4 {
            stream.clear();
            run.frame_into(&mut stream)
                .map_err(|error| error.to_string())?;
            let (size, pixels) = transmitted(&stream)?;
            let expected = render_changed(cols, rows, scale, tick)?;
            assert_eq!(
                size,
                (expected.width(), expected.height()),
                "{name} tick {tick}"
            );
            assert!(
                pixels == expected.as_rgba(),
                "{name} tick {tick}: sent pixels differ"
            );
        }
        let changed_bytes = stream.len();

        let name = format!("graphite-full-{scale}x");
        let case = graphite_case(&name).ok_or("no full case")?;
        let mut run = GraphiteRun::new(case, cols, rows)?;
        for tick in 0..3 {
            stream.clear();
            run.frame_into(&mut stream)
                .map_err(|error| error.to_string())?;
            let (size, pixels) = transmitted(&stream)?;
            let expected = render_full(cols, rows, scale, tick)?;
            assert_eq!(
                size,
                (expected.width(), expected.height()),
                "{name} tick {tick}"
            );
            assert!(
                pixels == expected.as_rgba(),
                "{name} tick {tick}: sent pixels differ"
            );
        }
        assert!(
            changed_bytes * 3 < stream.len(),
            "{scale}x: a changed frame sent {changed_bytes} bytes, a full frame {}",
            stream.len()
        );
    }
    Ok(())
}

#[test]
fn every_scene_and_repaint_is_registered_at_both_scales() {
    let scenes = [
        Scene::Raster,
        Scene::Widgets(Density::Monitor),
        Scene::Widgets(Density::Dense),
    ];
    for scene in scenes {
        for repaint in [Repaint::Changed, Repaint::Full] {
            for scale in [1, 2] {
                let matching = GRAPHITE_CASES
                    .iter()
                    .filter(|case| {
                        case.scene == scene && case.repaint == repaint && case.scale == scale
                    })
                    .count();
                assert_eq!(matching, 1, "{scene:?} {repaint:?} at {scale}x");
            }
        }
    }
    assert_eq!(GRAPHITE_CASES.len(), scenes.len() * 4);
    let mut names: Vec<_> = GRAPHITE_CASES.iter().map(|case| case.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), GRAPHITE_CASES.len());
}

#[test]
fn the_ceiling_gates_changed_frames_except_the_dense_worst_case_at_2x() {
    let gated: Vec<_> = GRAPHITE_CASES
        .iter()
        .filter(|case| case.ceiling_gated())
        .map(|case| case.name)
        .collect();
    assert_eq!(
        gated,
        [
            "graphite-changed-1x",
            "graphite-changed-2x",
            "graphite-monitor-changed-1x",
            "graphite-monitor-changed-2x",
            "graphite-dense-changed-1x",
        ]
    );
}
