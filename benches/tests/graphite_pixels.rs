// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use graphite_core::{DARK, DEFAULT_ACCENT, PixelBuffer, Rect, Rgba, accent};
use lunatui_core::{ClusterTable, Color, DecodedGlyph, Grid, Rect as CellRect};
use lunatui_term::{PixelLayer, PixelMode};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Image, ImageFit, ImageId, ImageSource, RenderCx, Widget};

const WIDTH: usize = 320;
const HEIGHT: usize = 96;
const STATUS_HEIGHT: usize = 20;
const COMMITTED: &[u8] = include_bytes!("../../examples/data/graphite-status-palette.rgba");
const FIXTURE: &str = "examples/data/graphite-status-palette.rgba";
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn solid(color: [u8; 3]) -> Rgba {
    Rgba::new(color[0], color[1], color[2], 255)
}

fn tinted(color: [u8; 3], alpha: u8) -> Rgba {
    Rgba::new(color[0], color[1], color[2], alpha)
}

fn frame() -> Result<PixelBuffer, String> {
    let tokens = &DARK;
    let mut buffer = PixelBuffer::new(WIDTH, HEIGHT, solid(tokens.ground))
        .map_err(|error| format!("{error:?}"))?;
    let tint = accent(tokens, DEFAULT_ACCENT);
    let panel = Rect::new(24, 6, 272, 62);
    buffer.fill_rect(panel, solid(tokens.panel));
    buffer.stroke_rect(panel, 1, solid(tokens.field_line));
    let input = Rect::new(32, 12, 256, 16);
    buffer.fill_rect(input, solid(tokens.field));
    buffer.stroke_rect(input, 1, solid(tokens.field_line));
    buffer.fill_rect(Rect::new(38, 16, 2, 8), solid(tint));
    for row in 0..3 {
        let top = 33 + row * 11;
        let line = Rect::new(32, top, 256, 10);
        if row == 0 {
            buffer.fill_rect(line, solid(tokens.tab_active));
            buffer.fill_rect(Rect::new(32, top, 2, 10), solid(tint));
        } else if row == 1 {
            buffer.fill_rect(line, tinted(tint, 40));
        }
        buffer.fill_rect(Rect::new(40, top + 3, 80 + row * 30, 4), solid(tokens.text));
        let key = Rect::new(258, top + 1, 26, 8);
        buffer.fill_rect(key, solid(tokens.key));
        buffer.stroke_rect(key, 1, solid(tokens.key_line));
    }
    let status = Rect::new(0, HEIGHT - STATUS_HEIGHT, WIDTH, STATUS_HEIGHT);
    buffer.fill_rect(status, solid(tokens.status_bar));
    buffer.fill_rect(
        Rect::new(0, HEIGHT - STATUS_HEIGHT, WIDTH, 1),
        solid(tokens.status_line),
    );
    for (index, color) in [tokens.working, tokens.attention, tokens.idle]
        .into_iter()
        .enumerate()
    {
        let left = 10 + index * 56;
        buffer.fill_rect(Rect::new(left, HEIGHT - 13, 6, 6), solid(color));
        buffer.fill_rect(
            Rect::new(left + 11, HEIGHT - 12, 30, 4),
            solid(tokens.muted),
        );
    }
    let chip = Rect::new(WIDTH - 70, HEIGHT - 14, 60, 8);
    buffer.fill_rect(chip, tinted(tokens.chip_active, 160));
    buffer.stroke_rect(chip, 1, solid(tint));
    Ok(buffer)
}

fn source(bytes: &[u8]) -> Result<ImageSource, String> {
    ImageSource::rgba8(
        u32::try_from(WIDTH).map_err(|error| error.to_string())?,
        u32::try_from(HEIGHT).map_err(|error| error.to_string())?,
        bytes.to_vec().into_boxed_slice(),
    )
    .map_err(|error| error.to_string())
}

fn base64_decode(text: &str) -> Result<Vec<u8>, String> {
    let body = text
        .strip_suffix("==")
        .or_else(|| text.strip_suffix('='))
        .unwrap_or(text);
    if body.len() % 4 == 1 {
        return Err(format!("{} symbols cannot be base64", body.len()));
    }
    let mut out = Vec::new();
    let mut bits = 0_u32;
    let mut count = 0;
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

fn emitted_stream(pixels: &[u8]) -> Result<String, String> {
    let mut grid = Grid::new(100, 32);
    let mut clusters = ClusterTable::new();
    let theme = Theme::builtin(Variant::Dark);
    let mut layer = PixelLayer::new(PixelMode::Kitty);
    layer.begin_frame();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme).image_sink(&mut layer);
    Image::new(source(pixels)?)
        .id(ImageId::new(81).ok_or("zero image id")?)
        .fit(ImageFit::Contain)
        .render(CellRect::new(4, 4, 92, 25), &mut cx);
    String::from_utf8(layer.finish_frame().to_vec()).map_err(|error| error.to_string())
}

fn transmitted_pixels(stream: &str) -> Result<Vec<u8>, String> {
    let mut payload = Vec::new();
    let mut transmitted = 0;
    for command in stream.split("\x1b_G").skip(1) {
        let body = command
            .split("\x1b\\")
            .next()
            .ok_or("unterminated command")?;
        let (keys, data) = body.split_once(';').unwrap_or((body, ""));
        if keys.contains("a=p") {
            continue;
        }
        if keys.contains("a=t") {
            transmitted += 1;
            if !keys.contains("f=32") || !keys.contains("s=320,v=96") {
                return Err(format!("not a 320x96 RGBA transmit: {keys}"));
            }
        }
        payload.extend(base64_decode(data)?);
    }
    if transmitted == 1 {
        Ok(payload)
    } else {
        Err(format!("{transmitted} transmit commands"))
    }
}

#[test]
fn the_committed_fixture_is_the_frame_graphite_draws() -> Result<(), String> {
    let buffer = frame()?;
    if std::env::var_os("GRAPHITE_PIXELS_BLESS").is_some() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(FIXTURE);
        std::fs::write(path, buffer.as_rgba()).map_err(|error| error.to_string())?;
    }
    assert_eq!(buffer.as_rgba().len(), WIDTH * HEIGHT * 4);
    assert!(
        COMMITTED == buffer.as_rgba(),
        "{FIXTURE} differs from the Graphite frame; rerun with GRAPHITE_PIXELS_BLESS=1"
    );
    Ok(())
}

#[test]
fn the_kitty_stream_carries_every_graphite_pixel_unchanged() -> Result<(), String> {
    let buffer = frame()?;
    let stream = emitted_stream(buffer.as_rgba())?;
    let payload = transmitted_pixels(&stream)?;
    assert_eq!(payload.len(), WIDTH * HEIGHT * 4);
    assert!(
        payload == buffer.as_rgba(),
        "stream pixels differ from the Graphite frame"
    );
    Ok(())
}

#[test]
fn the_decoder_rejects_a_corrupted_stream_and_a_changed_pixel_is_seen() -> Result<(), String> {
    let buffer = frame()?;
    let stream = emitted_stream(buffer.as_rgba())?;
    let data_start = stream.find(';').ok_or("no payload")? + 1;
    let letter = stream[data_start..]
        .find('A')
        .map(|at| at + data_start)
        .ok_or("no A in the payload")?;
    let mut invalid = stream.clone();
    invalid.replace_range(letter..=letter, "!");
    assert!(
        transmitted_pixels(&invalid).is_err(),
        "an invalid symbol was accepted"
    );
    let mut padded = stream.clone();
    padded.replace_range(letter..=letter, "=");
    assert!(
        transmitted_pixels(&padded).is_err(),
        "padding inside the payload was accepted"
    );
    let mut changed = buffer.as_rgba().to_vec();
    changed[WIDTH * 4 * 40 + 4 * 100] ^= 0x01;
    let other = transmitted_pixels(&emitted_stream(&changed)?)?;
    assert!(other != buffer.as_rgba(), "a changed pixel went unseen");
    assert_eq!(
        other
            .iter()
            .zip(buffer.as_rgba())
            .filter(|(a, b)| a != b)
            .count(),
        1
    );
    assert_eq!(base64_decode("QQ==")?, b"A");
    assert!(base64_decode("Q").is_err());
    Ok(())
}

#[test]
fn the_cell_fallback_shows_the_same_graphite_pixels_as_half_blocks() -> Result<(), String> {
    let buffer = frame()?;
    let columns = u16::try_from(WIDTH).map_err(|error| error.to_string())?;
    let rows = u16::try_from(HEIGHT / 2).map_err(|error| error.to_string())?;
    let mut grid = Grid::new(columns, rows);
    let mut clusters = ClusterTable::new();
    let theme = Theme::builtin(Variant::Dark);
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Image::new(source(buffer.as_rgba())?)
        .id(ImageId::new(81).ok_or("zero image id")?)
        .fit(ImageFit::Stretch)
        .render(CellRect::new(0, 0, columns, rows), &mut cx);
    let mut checked = 0_usize;
    for row in 0..rows {
        for column in 0..columns {
            let cell = grid.get(column, row).ok_or("cell missing")?;
            assert_eq!(cell.glyph().decode(), DecodedGlyph::Inline('▀'));
            let top = buffer
                .pixel(usize::from(column), usize::from(row) * 2)
                .ok_or("top pixel missing")?;
            let bottom = buffer
                .pixel(usize::from(column), usize::from(row) * 2 + 1)
                .ok_or("bottom pixel missing")?;
            assert_eq!(
                cell.style().fg,
                Color::Rgb(top.r, top.g, top.b),
                "({column},{row}) top"
            );
            assert_eq!(
                cell.style().bg,
                Color::Rgb(bottom.r, bottom.g, bottom.b),
                "({column},{row}) bottom"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, WIDTH * HEIGHT / 2);
    Ok(())
}
