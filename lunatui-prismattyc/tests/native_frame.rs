// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Color, Grid, NativeAttrs, Rect, Style};
use lunatui_prismattyc::{
    ACTIVE_FLAG_ALPHA, AttributeMode, EncodeError, FRAME_HEADER_BYTES, FrameEncoder,
    MAX_FRAME_AXIS, SLOT_BYTES, ansi_fallback_repaint_prefix,
};

struct Frame {
    columns: u32,
    rows: u32,
    generation: u64,
    sequence: u64,
    cursor: Option<(u32, u32)>,
    cells: Vec<[u32; 4]>,
    clusters: Vec<String>,
    attributes: Vec<[u8; 4]>,
}

fn ensure(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}

fn word32(bytes: &[u8], at: usize) -> Result<u32, String> {
    let slice = bytes.get(at..at + 4).ok_or("short read")?;
    Ok(u32::from_le_bytes(
        slice.try_into().map_err(|_| "short read")?,
    ))
}

fn word16(bytes: &[u8], at: usize) -> Result<u16, String> {
    let slice = bytes.get(at..at + 2).ok_or("short read")?;
    Ok(u16::from_le_bytes(
        slice.try_into().map_err(|_| "short read")?,
    ))
}

fn word64(bytes: &[u8], at: usize) -> Result<u64, String> {
    let slice = bytes.get(at..at + 8).ok_or("short read")?;
    Ok(u64::from_le_bytes(
        slice.try_into().map_err(|_| "short read")?,
    ))
}

fn usize_of(value: u32) -> Result<usize, String> {
    usize::try_from(value).map_err(|_| "overflow".to_owned())
}

fn align8(value: usize) -> usize {
    value.div_ceil(8) * 8
}

fn color_valid(bits: u32) -> bool {
    match bits >> 30 {
        0 => bits == 0,
        1 => bits & 0x3FFF_FF00 == 0,
        2 => bits & 0x3F00_0000 == 0,
        _ => false,
    }
}

fn field(value: u32, shift: u32) -> u32 {
    (value >> shift) & 3
}

struct Layout {
    columns: usize,
    rows: usize,
    count: usize,
    clusters_at: usize,
    cluster_count: usize,
    blob_at: usize,
    blob_len: usize,
    attribute_count: usize,
    attribute_at: usize,
}

fn parse_layout(bytes: &[u8], minor: u16) -> Result<Layout, String> {
    ensure(bytes.len() >= FRAME_HEADER_BYTES, "header")?;
    ensure(bytes.len() <= SLOT_BYTES, "slot size")?;
    ensure(usize_of(word32(bytes, 0)?)? == bytes.len(), "used bytes")?;
    ensure(
        usize::from(word16(bytes, 4)?) == FRAME_HEADER_BYTES,
        "header bytes",
    )?;
    ensure(word16(bytes, 6)? == 1, "flags")?;
    let (columns, rows) = (usize_of(word32(bytes, 24)?)?, usize_of(word32(bytes, 28)?)?);
    ensure(
        (1..=4096).contains(&columns) && (1..=4096).contains(&rows),
        "axis",
    )?;
    let count = usize_of(word32(bytes, 32)?)?;
    ensure(count == columns * rows && count <= 32_768, "cells")?;
    ensure(
        usize_of(word32(bytes, 36)?)? == FRAME_HEADER_BYTES,
        "cells offset",
    )?;
    let cluster_count = usize_of(word32(bytes, 40)?)?;
    ensure(cluster_count <= count, "cluster count")?;
    let clusters_at = FRAME_HEADER_BYTES + count * 16;
    ensure(
        usize_of(word32(bytes, 44)?)? == clusters_at,
        "clusters offset",
    )?;
    let blob_len = usize_of(word32(bytes, 48)?)?;
    ensure(blob_len <= 262_144, "blob size")?;
    let blob_at = align8(clusters_at + cluster_count * 16);
    ensure(usize_of(word32(bytes, 52)?)? == blob_at, "blob offset")?;
    ensure(
        word32(bytes, 56)? == 0,
        "full snapshot has no damage records",
    )?;
    let damage_at = align8(blob_at + blob_len);
    ensure(usize_of(word32(bytes, 60)?)? == damage_at, "damage offset")?;
    let attribute_count = usize_of(word32(bytes, 76)?)?;
    let attribute_at = usize_of(word32(bytes, 80)?)?;
    let tail = bytes.get(84..FRAME_HEADER_BYTES).ok_or("tail")?;
    ensure(tail.iter().all(|&byte| byte == 0), "reserved header bytes")?;
    let used = if minor >= 1 && attribute_count > 0 {
        ensure(attribute_count <= count, "attribute count")?;
        ensure(attribute_at == damage_at, "attribute offset")?;
        damage_at + attribute_count * 8
    } else {
        ensure(
            attribute_count == 0 && attribute_at == 0,
            "no attribute section in 1.0",
        )?;
        damage_at
    };
    ensure(used == bytes.len(), "section sizes add up")?;
    let padding = [
        (clusters_at + cluster_count * 16, blob_at),
        (blob_at + blob_len, damage_at),
    ];
    for (from, to) in padding {
        ensure(bytes[from..to].iter().all(|&b| b == 0), "padding")?;
    }
    Ok(Layout {
        columns,
        rows,
        count,
        clusters_at,
        cluster_count,
        blob_at,
        blob_len,
        attribute_count,
        attribute_at,
    })
}

fn parse_cursor(bytes: &[u8], layout: &Layout) -> Result<Option<(u32, u32)>, String> {
    let visible = word32(bytes, 72)?;
    ensure(visible <= 1, "cursor flags")?;
    let (x, y) = (word32(bytes, 64)?, word32(bytes, 68)?);
    if visible == 1 {
        ensure(
            usize_of(x)? < layout.columns && usize_of(y)? < layout.rows,
            "cursor position",
        )?;
        Ok(Some((x, y)))
    } else {
        ensure(x == 0 && y == 0, "hidden cursor")?;
        Ok(None)
    }
}

fn parse_clusters(bytes: &[u8], layout: &Layout) -> Result<Vec<String>, String> {
    let mut clusters = Vec::new();
    let mut packed = 0;
    for index in 0..layout.cluster_count {
        let at = layout.clusters_at + index * 16;
        let (offset, len) = (
            usize_of(word32(bytes, at)?)?,
            usize_of(word32(bytes, at + 4)?)?,
        );
        ensure(offset == packed && len > 0 && len <= 1024, "blob packing")?;
        ensure(matches!(bytes[at + 8], 1 | 2), "cluster width")?;
        ensure(
            bytes[at + 9..at + 16].iter().all(|&b| b == 0),
            "cluster reserved",
        )?;
        let start = layout.blob_at + offset;
        let text = std::str::from_utf8(&bytes[start..start + len]).map_err(|_| "utf-8")?;
        ensure(!text.chars().any(char::is_control), "cluster control")?;
        clusters.push(text.to_owned());
        packed += len;
    }
    ensure(packed == layout.blob_len, "blob has no gaps")?;
    Ok(clusters)
}

fn parse_attributes(bytes: &[u8], layout: &Layout) -> Result<Vec<[u8; 4]>, String> {
    let mut attributes: Vec<[u8; 4]> = Vec::new();
    for index in 0..layout.attribute_count {
        let at = layout.attribute_at + index * 8;
        let record = &bytes[at..at + 8];
        ensure(record[4..].iter().all(|&b| b == 0), "attribute reserved")?;
        let values = [record[0], record[1], record[2], record[3]];
        let (dx, dy) = (
            i8::from_le_bytes([record[2]]),
            i8::from_le_bytes([record[3]]),
        );
        ensure(dx.abs() <= 7 && dy.abs() <= 7, "offset range")?;
        ensure(values != [255, 255, 0, 0], "default record")?;
        ensure(!attributes.contains(&values), "dedup")?;
        attributes.push(values);
    }
    Ok(attributes)
}

fn parse_cells(
    bytes: &[u8],
    layout: &Layout,
    minor: u16,
    attributes: usize,
) -> Result<Vec<[u32; 4]>, String> {
    let clusters = u32::try_from(layout.cluster_count).map_err(|_| "count")?;
    let mut cells = Vec::new();
    for index in 0..layout.count {
        let at = FRAME_HEADER_BYTES + index * 16;
        let cell = [
            word32(bytes, at)?,
            word32(bytes, at + 4)?,
            word32(bytes, at + 8)?,
            word32(bytes, at + 12)?,
        ];
        let glyph = cell[0];
        if glyph & 0x8000_0000 != 0 {
            ensure(glyph & 0x7FFF_FFFF < clusters, "cluster ref")?;
        } else {
            let scalar = char::from_u32(glyph).is_some_and(|c| !c.is_control());
            ensure(glyph == 0x0011_0000 || scalar, "scalar")?;
        }
        ensure(color_valid(cell[1]) && color_valid(cell[2]), "color")?;
        let meta = cell[3];
        ensure(
            (meta >> 3) & 7 <= 5 && meta & (1 << 11) == 0,
            "meta attributes",
        )?;
        ensure(field(meta, 14) == 0, "meta flags")?;
        ensure(field(meta, 12) <= 2, "width")?;
        let record = usize_of(meta >> 16)?;
        ensure(
            record <= if minor == 0 { 0 } else { attributes },
            "attribute index in range",
        )?;
        cells.push(cell);
    }
    Ok(cells)
}

fn check_pairs(cells: &[[u32; 4]], columns: usize) -> Result<(), String> {
    for (index, cell) in cells.iter().enumerate() {
        let column = index % columns;
        match field(cell[3], 12) {
            2 => {
                let next = cells.get(index + 1).ok_or("wide pair")?;
                ensure(column + 1 < columns && field(next[3], 12) == 0, "wide pair")?;
            }
            0 => {
                let lead = index
                    .checked_sub(1)
                    .and_then(|at| cells.get(at))
                    .ok_or("lead")?;
                ensure(
                    column > 0 && field(lead[3], 12) == 2,
                    "continuation follows wide",
                )?;
                ensure(cell[0] == 0x0011_0000, "continuation glyph")?;
                ensure(
                    cell[3] >> 16 == lead[3] >> 16,
                    "continuation shares the lead's record",
                )?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn parse(bytes: &[u8], minor: u16) -> Result<Frame, String> {
    let layout = parse_layout(bytes, minor)?;
    let cursor = parse_cursor(bytes, &layout)?;
    let clusters = parse_clusters(bytes, &layout)?;
    let attributes = parse_attributes(bytes, &layout)?;
    let cells = parse_cells(bytes, &layout, minor, attributes.len())?;
    check_pairs(&cells, layout.columns)?;
    Ok(Frame {
        columns: u32::try_from(layout.columns).map_err(|_| "columns")?,
        rows: u32::try_from(layout.rows).map_err(|_| "rows")?,
        generation: word64(bytes, 16)?,
        sequence: word64(bytes, 8)?,
        cursor,
        cells,
        clusters,
        attributes,
    })
}

fn offered() -> FrameEncoder {
    let mut encoder = FrameEncoder::new();
    encoder.offer_alpha(true);
    encoder
}

fn glass() -> NativeAttrs {
    NativeAttrs::new(255, 128, 0, 0)
}

fn shifted() -> NativeAttrs {
    NativeAttrs::new(200, 255, 3, -2)
}

fn scene() -> (Grid, ClusterTable) {
    let mut grid = Grid::new(10, 3);
    let mut clusters = ClusterTable::new();
    let style = Style::new()
        .fg(Color::Rgb(10, 20, 30))
        .bg(Color::Indexed(4));
    grid.put_str(0, 0, style, "ab漢e\u{301}", &mut clusters);
    grid.put_str(0, 1, Style::new(), "tail", &mut clusters);
    grid.set_native(1, 0, glass());
    grid.set_native(2, 0, shifted());
    grid.set_native(4, 0, glass());
    grid.set_native(0, 1, shifted());
    (grid, clusters)
}

fn encode(
    encoder: &mut FrameEncoder,
    grid: &Grid,
    clusters: &ClusterTable,
) -> Result<Vec<u8>, String> {
    encoder
        .encode(grid, clusters, 7, 3, Some((0, 0)))
        .map(<[u8]>::to_vec)
        .map_err(|error| error.to_string())
}

#[test]
fn not_negotiated_frames_are_plain_abi_1_0() -> Result<(), String> {
    let (grid, clusters) = scene();
    let mut encoder = FrameEncoder::new();
    assert_eq!(encoder.mode(), AttributeMode::Stripped);
    let bytes = encode(&mut encoder, &grid, &clusters)?;
    let frame = parse(&bytes, 0)?;
    assert_eq!(frame.attributes.len(), 0);
    assert!(frame.cells.iter().all(|cell| cell[3] >> 16 == 0));
    assert_eq!((frame.columns, frame.rows), (10, 3));
    assert_eq!((frame.generation, frame.sequence), (7, 3));
    assert_eq!(frame.cursor, Some((0, 0)));
    Ok(())
}

#[test]
fn not_negotiated_bytes_match_a_grid_that_never_had_attributes() -> Result<(), String> {
    let (grid, clusters) = scene();
    let mut encoder = FrameEncoder::new();
    let with = encode(&mut encoder, &grid, &clusters)?;
    let without = encode(&mut encoder, &grid.without_native(), &clusters)?;
    assert_eq!(with, without);
    Ok(())
}

#[test]
fn a_reply_without_the_flag_keeps_attributes_stripped() -> Result<(), String> {
    let (grid, clusters) = scene();
    let mut encoder = FrameEncoder::new();
    assert_eq!(encoder.affirm_alpha(0), AttributeMode::Stripped);
    let bytes = encode(&mut encoder, &grid, &clusters)?;
    assert_eq!(parse(&bytes, 0)?.attributes.len(), 0);
    Ok(())
}

#[test]
fn unknown_flag_bits_do_not_affirm_the_feature() {
    let mut encoder = offered();
    assert_eq!(
        encoder.affirm_alpha(ACTIVE_FLAG_ALPHA | 2),
        AttributeMode::Stripped
    );
    assert_eq!(encoder.affirm_alpha(2), AttributeMode::Stripped);
    assert_eq!(
        encoder.affirm_alpha(ACTIVE_FLAG_ALPHA),
        AttributeMode::Native
    );
}

#[test]
fn negotiated_frames_carry_the_attributes() -> Result<(), String> {
    let (grid, clusters) = scene();
    let mut encoder = offered();
    assert_eq!(
        encoder.affirm_alpha(ACTIVE_FLAG_ALPHA),
        AttributeMode::Native
    );
    let bytes = encode(&mut encoder, &grid, &clusters)?;
    let frame = parse(&bytes, 1)?;
    assert_eq!(frame.attributes.len(), 2);
    for y in 0..3u16 {
        for x in 0..10u16 {
            let cell = frame.cells[usize::from(y) * 10 + usize::from(x)];
            let index = usize::try_from(cell[3] >> 16).map_err(|_| "index")?;
            let expected = grid.native(x, y);
            if expected.is_default() {
                assert_eq!(index, 0, "cell {x},{y}");
            } else {
                let record = frame.attributes[index - 1];
                let decoded = NativeAttrs::new(
                    record[0],
                    record[1],
                    i8::from_le_bytes([record[2]]),
                    i8::from_le_bytes([record[3]]),
                );
                assert_eq!(decoded, expected, "cell {x},{y}");
            }
        }
    }
    Ok(())
}

#[test]
fn equal_attributes_share_one_record_and_a_wide_pair_shares_its_lead() -> Result<(), String> {
    let (grid, clusters) = scene();
    let mut encoder = offered();
    let _ = encoder.affirm_alpha(ACTIVE_FLAG_ALPHA);
    let frame = parse(&encode(&mut encoder, &grid, &clusters)?, 1)?;
    assert_eq!(frame.cells[1][3] >> 16, frame.cells[4][3] >> 16);
    assert_eq!(frame.cells[2][3] >> 16, frame.cells[3][3] >> 16);
    assert_ne!(frame.cells[2][3] >> 16, 0);
    Ok(())
}

#[test]
fn negotiated_and_stripped_frames_agree_on_everything_but_the_attributes() -> Result<(), String> {
    let (grid, clusters) = scene();
    let mut plain = FrameEncoder::new();
    let mut native = offered();
    let _ = native.affirm_alpha(ACTIVE_FLAG_ALPHA);
    let stripped = parse(&encode(&mut plain, &grid, &clusters)?, 0)?;
    let rich = parse(&encode(&mut native, &grid, &clusters)?, 1)?;
    assert_eq!(stripped.clusters, rich.clusters);
    for (a, b) in stripped.cells.iter().zip(&rich.cells) {
        assert_eq!(a[..3], b[..3]);
        assert_eq!(a[3] & 0xFFFF, b[3] & 0xFFFF);
    }
    Ok(())
}

#[test]
fn a_negotiated_frame_without_attributes_is_byte_identical_to_abi_1_0() -> Result<(), String> {
    let (grid, clusters) = scene();
    let bare = grid.without_native();
    let mut plain = FrameEncoder::new();
    let mut native = offered();
    let _ = native.affirm_alpha(ACTIVE_FLAG_ALPHA);
    let old = encode(&mut plain, &bare, &clusters)?;
    let new = encode(&mut native, &bare, &clusters)?;
    assert_eq!(old, new);
    assert_eq!(parse(&new, 0)?.attributes.len(), 0);
    assert_eq!(parse(&new, 1)?.attributes.len(), 0);
    Ok(())
}

#[test]
fn a_1_0_host_rejects_a_frame_with_attribute_records() -> Result<(), String> {
    let (grid, clusters) = scene();
    let mut encoder = offered();
    let _ = encoder.affirm_alpha(ACTIVE_FLAG_ALPHA);
    let bytes = encode(&mut encoder, &grid, &clusters)?;
    assert!(parse(&bytes, 0).is_err());
    Ok(())
}

#[test]
fn clusters_are_copied_once_and_remapped() -> Result<(), String> {
    let mut grid = Grid::new(6, 1);
    let mut clusters = ClusterTable::new();
    grid.put_str(0, 0, Style::new(), "e\u{301}x e\u{301}", &mut clusters);
    let mut encoder = FrameEncoder::new();
    let frame = parse(&encode(&mut encoder, &grid, &clusters)?, 0)?;
    assert_eq!(frame.clusters, vec!["e\u{301}".to_owned()]);
    assert_eq!(frame.cells[0][0], 0x8000_0000);
    assert_eq!(frame.cells[3][0], 0x8000_0000);
    Ok(())
}

#[test]
fn failure_ends_native_output_and_asks_for_an_ansi_repaint() {
    let (grid, clusters) = scene();
    let mut encoder = offered();
    let _ = encoder.affirm_alpha(ACTIVE_FLAG_ALPHA);
    let big = Grid::new(MAX_FRAME_AXIS, 9);
    assert_eq!(
        encoder.encode(&big, &clusters, 7, 4, None).err(),
        Some(EncodeError::Geometry)
    );
    assert_eq!(encoder.fail(), ansi_fallback_repaint_prefix());
    assert!(encoder.is_detached());
    assert_eq!(encoder.mode(), AttributeMode::Stripped);
    assert_eq!(
        encoder.encode(&grid, &clusters, 7, 5, None).err(),
        Some(EncodeError::Detached)
    );
    assert_eq!(
        encoder.affirm_alpha(ACTIVE_FLAG_ALPHA),
        AttributeMode::Stripped
    );
}

#[test]
fn the_largest_frame_fits_and_a_bigger_one_is_refused() {
    let mut grid = Grid::new(128, 256);
    let mut clusters = ClusterTable::new();
    for y in 0..256u16 {
        grid.put_str(0, y, Style::new(), "x", &mut clusters);
    }
    let mut encoder = FrameEncoder::new();
    assert!(encoder.encode(&grid, &clusters, 1, 1, None).is_ok());
    let wide = Grid::new(4096, 9);
    assert_eq!(
        encoder.encode(&wide, &clusters, 1, 1, None).err(),
        Some(EncodeError::Geometry)
    );
}

#[test]
fn bad_inputs_are_refused() {
    let (grid, clusters) = scene();
    let mut encoder = FrameEncoder::new();
    assert_eq!(
        encoder.encode(&grid, &clusters, 0, 1, None).err(),
        Some(EncodeError::Generation)
    );
    assert_eq!(
        encoder.encode(&grid, &clusters, 1, 1, Some((10, 0))).err(),
        Some(EncodeError::Cursor)
    );
    assert_eq!(
        encoder
            .encode(&Grid::new(0, 0), &clusters, 1, 1, None)
            .err(),
        Some(EncodeError::Geometry)
    );
}

#[test]
fn theme_tokens_need_a_resolved_color() -> Result<(), String> {
    let mut grid = Grid::new(4, 1);
    let mut clusters = ClusterTable::new();
    grid.put_str(0, 0, Style::new().fg(Color::Token(1)), "ab", &mut clusters);
    let mut encoder = FrameEncoder::new();
    assert_eq!(
        encoder.encode(&grid, &clusters, 1, 1, None).err(),
        Some(EncodeError::UnresolvedToken)
    );
    encoder.set_token_palette(vec![Color::Default, Color::Rgb(1, 2, 3)]);
    let frame = parse(&encode(&mut encoder, &grid, &clusters)?, 0)?;
    assert_eq!(frame.cells[0][1], (2 << 30) | 0x01_02_03);
    encoder.set_token_palette(vec![Color::Default, Color::Token(0)]);
    assert_eq!(
        encoder.encode(&grid, &clusters, 1, 1, None).err(),
        Some(EncodeError::UnresolvedToken)
    );
    Ok(())
}

#[test]
fn a_full_attribute_table_still_gives_a_valid_frame() -> Result<(), String> {
    let mut grid = Grid::new(200, 100);
    let area = Rect::new(0, 0, 200, 100);
    let mut clusters = ClusterTable::new();
    grid.fill(area, Style::new(), 'x', &mut clusters);
    for y in 0..100u16 {
        for x in 0..200u16 {
            let n = u32::from(y) * 200 + u32::from(x);
            let attrs = NativeAttrs::new(
                u8::try_from(n % 254).unwrap_or(0),
                u8::try_from((n / 254) % 255).unwrap_or(0),
                i8::try_from(n % 15).unwrap_or(0) - 7,
                0,
            );
            grid.set_native(x, y, attrs);
        }
    }
    let mut encoder = offered();
    let _ = encoder.affirm_alpha(ACTIVE_FLAG_ALPHA);
    let frame = parse(&encode(&mut encoder, &grid, &clusters)?, 1)?;
    assert!(frame.attributes.len() <= 20_000);
    Ok(())
}

#[test]
fn geometry_limits_are_exact() {
    let clusters = ClusterTable::new();
    let mut encoder = FrameEncoder::new();
    for (width, height, ok) in [
        (4096u16, 8u16, true),
        (4096, 1, true),
        (1, 4096, true),
        (4096, 9, false),
        (4097, 1, false),
        (1, 4097, false),
        (0, 5, false),
        (5, 0, false),
        (181, 181, true),
        (182, 181, false),
    ] {
        let grid = Grid::new(width, height);
        let result = encoder.encode(&grid, &clusters, 1, 1, None);
        assert_eq!(result.is_ok(), ok, "{width}x{height}");
    }
}

#[test]
fn the_cursor_may_sit_on_the_last_cell_only() {
    let clusters = ClusterTable::new();
    let grid = Grid::new(5, 3);
    let mut encoder = FrameEncoder::new();
    assert!(encoder.encode(&grid, &clusters, 1, 1, Some((4, 2))).is_ok());
    assert_eq!(
        encoder.encode(&grid, &clusters, 1, 1, Some((5, 0))).err(),
        Some(EncodeError::Cursor)
    );
    assert_eq!(
        encoder.encode(&grid, &clusters, 1, 1, Some((0, 3))).err(),
        Some(EncodeError::Cursor)
    );
}

#[test]
fn every_error_has_its_own_message() {
    let errors = [
        EncodeError::Geometry,
        EncodeError::TooLarge,
        EncodeError::Generation,
        EncodeError::Cursor,
        EncodeError::Unsupported,
        EncodeError::Control,
        EncodeError::Malformed,
        EncodeError::UnresolvedToken,
        EncodeError::Detached,
    ];
    let messages: Vec<String> = errors.iter().map(ToString::to_string).collect();
    for (index, message) in messages.iter().enumerate() {
        assert_ne!(message, "");
        assert_eq!(
            messages.iter().filter(|other| *other == message).count(),
            1,
            "{index}"
        );
    }
}

#[test]
fn distinct_clusters_get_consecutive_ids_and_exact_color_words() -> Result<(), String> {
    let mut grid = Grid::new(6, 1);
    let mut clusters = ClusterTable::new();
    let style = Style::new()
        .fg(Color::Rgb(10, 20, 30))
        .bg(Color::Indexed(4));
    grid.put_str(0, 0, style, "e\u{301}a\u{301}e\u{301}", &mut clusters);
    let mut encoder = FrameEncoder::new();
    let frame = parse(&encode(&mut encoder, &grid, &clusters)?, 0)?;
    assert_eq!(frame.clusters.len(), 2);
    assert_eq!(frame.cells[0][0], 0x8000_0000);
    assert_eq!(frame.cells[1][0], 0x8000_0001);
    assert_eq!(frame.cells[2][0], 0x8000_0000);
    assert_eq!(frame.cells[0][1], (2 << 30) | 0x0A_14_1E);
    assert_eq!(frame.cells[0][2], (1 << 30) | 4);
    Ok(())
}

#[test]
fn cluster_text_is_limited_to_1024_bytes() {
    let exact = format!("e{}\u{20DD}", "\u{301}".repeat(510));
    let over = format!("e{}\u{20DD}\u{301}", "\u{301}".repeat(510));
    assert_eq!((exact.len(), over.len()), (1024, 1026));
    let mut encoder = FrameEncoder::new();
    for (text, ok) in [(&exact, true), (&over, false)] {
        let mut clusters = ClusterTable::new();
        let mut grid = Grid::new(4, 1);
        grid.put_str(0, 0, Style::new(), text, &mut clusters);
        let result = encoder.encode(&grid, &clusters, 1, 1, None);
        assert_eq!(result.is_ok(), ok);
        if !ok {
            assert_eq!(result.err(), Some(EncodeError::TooLarge));
        }
    }
}

#[test]
fn a_cluster_the_table_does_not_hold_is_refused() {
    let mut clusters = ClusterTable::new();
    let mut grid = Grid::new(4, 1);
    grid.put_str(0, 0, Style::new(), "e\u{301}", &mut clusters);
    let mut encoder = FrameEncoder::new();
    assert_eq!(
        encoder
            .encode(&grid, &ClusterTable::new(), 1, 1, None)
            .err(),
        Some(EncodeError::Malformed)
    );
}

#[test]
fn the_mode_and_detached_state_are_reported() {
    let mut encoder = offered();
    assert!(!encoder.is_detached());
    let _ = encoder.affirm_alpha(ACTIVE_FLAG_ALPHA);
    assert_eq!(encoder.mode(), AttributeMode::Native);
    let _ = encoder.fail();
    assert!(encoder.is_detached());
}

fn cluster_text(index: u32) -> Result<String, String> {
    let base = char::from_u32(0x4E00 + index).ok_or("base")?;
    Ok(format!("{base}{}\u{20DD}", "\u{301}".repeat(509)))
}

fn grid_of_clusters(count: u32, clusters: &mut ClusterTable) -> Result<Grid, String> {
    let mut grid = Grid::new(4096, 8);
    for index in 0..count {
        let text = cluster_text(index)?;
        assert_eq!(text.len(), 1024);
        let column = u16::try_from(index * 2).map_err(|_| "column")?;
        grid.put_str(column, 0, Style::new(), &text, clusters);
    }
    Ok(grid)
}

#[test]
fn the_cluster_blob_is_limited_to_256_kib() -> Result<(), String> {
    let mut encoder = FrameEncoder::new();
    for (count, ok) in [(256, true), (257, false)] {
        let mut clusters = ClusterTable::new();
        let grid = grid_of_clusters(count, &mut clusters)?;
        let result = encoder.encode(&grid, &clusters, 1, 1, None);
        assert_eq!(result.is_ok(), ok, "{count} clusters");
        if !ok {
            assert_eq!(result.err(), Some(EncodeError::TooLarge));
        }
    }
    Ok(())
}

#[test]
fn a_frame_over_one_slot_is_refused_and_stripping_makes_it_fit() -> Result<(), String> {
    let mut clusters = ClusterTable::new();
    let mut grid = grid_of_clusters(256, &mut clusters)?;
    for y in 0..8u16 {
        for x in 0..4096u16 {
            let n = u32::from(y) * 4096 + u32::from(x);
            let attrs = NativeAttrs::new(
                u8::try_from(n % 254).unwrap_or(0),
                u8::try_from((n / 254) % 255).unwrap_or(0),
                0,
                0,
            );
            grid.set_native(x, y, attrs);
        }
    }
    let mut encoder = offered();
    assert!(encoder.encode(&grid, &clusters, 1, 1, None).is_ok());
    let _ = encoder.affirm_alpha(ACTIVE_FLAG_ALPHA);
    assert_eq!(
        encoder.encode(&grid, &clusters, 1, 1, None).err(),
        Some(EncodeError::TooLarge)
    );
    Ok(())
}

#[test]
fn without_an_offer_the_client_stays_on_abi_1_0() {
    let mut encoder = FrameEncoder::new();
    assert_eq!(encoder.hello_ring_minor(), 0);
    assert_eq!(
        encoder.affirm_alpha(ACTIVE_FLAG_ALPHA),
        AttributeMode::Stripped
    );
    encoder.offer_alpha(false);
    assert_eq!(encoder.hello_ring_minor(), 0);
    assert_eq!(
        encoder.affirm_alpha(ACTIVE_FLAG_ALPHA),
        AttributeMode::Stripped
    );
}

#[test]
fn an_offer_sends_ring_minor_1_and_resets_the_mode() {
    let mut encoder = offered();
    assert_eq!(encoder.hello_ring_minor(), 1);
    assert_eq!(encoder.mode(), AttributeMode::Stripped);
    assert_eq!(
        encoder.affirm_alpha(ACTIVE_FLAG_ALPHA),
        AttributeMode::Native
    );
    encoder.offer_alpha(true);
    assert_eq!(encoder.mode(), AttributeMode::Stripped);
}
