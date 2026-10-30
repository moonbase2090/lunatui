// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::collections::BTreeMap;

use lunatui_core::Rect;
use lunatui_image::{Fit, ImageId, ImageRequest, ImageSink, ImageSource};
use lunatui_term::{
    Caps, CellPixels, GraphicsProtocol, PixelImage, PixelLayer, PixelMode, Protocols,
};

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
#[derive(Debug)]
struct Command {
    keys: BTreeMap<String, String>,
    payload: Vec<u8>,
    cursor: Option<(u16, u16)>,
}

fn base64_decode(text: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut bits = 0_u32;
    let mut count = 0;
    for byte in text.bytes().filter(|byte| *byte != b'=') {
        let value = ALPHABET
            .iter()
            .position(|candidate| *candidate == byte)
            .ok_or_else(|| format!("byte {byte} is not base64"))?;
        bits = bits << 6 | u32::try_from(value).unwrap_or(0);
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push(u8::try_from((bits >> count) & 0xff).unwrap_or(0));
        }
    }
    Ok(out)
}
fn parse(stream: &[u8]) -> Result<Vec<Command>, String> {
    let text = std::str::from_utf8(stream).map_err(|error| error.to_string())?;
    let mut commands = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("\x1b_G") {
        let before = &rest[..start];
        if before.contains("\x1b_G") || (!before.is_empty() && !before.starts_with("\x1b[")) {
            return Err(format!("unexpected bytes before a command: {before:?}"));
        }
        let body_start = start + 3;
        let end = rest[body_start..]
            .find("\x1b\\")
            .ok_or("unterminated APC command")?
            + body_start;
        let body = &rest[body_start..end];
        let (keys_text, payload_text) = body.split_once(';').unwrap_or((body, ""));
        let keys = keys_text
            .split(',')
            .filter(|pair| !pair.is_empty())
            .map(|pair| {
                pair.split_once('=')
                    .map(|(key, value)| (key.to_owned(), value.to_owned()))
                    .ok_or_else(|| format!("key without value: {pair}"))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        commands.push(Command {
            keys,
            payload: base64_decode(payload_text)?,
            cursor: before.rsplit_once("\x1b[").and_then(|(_, sequence)| {
                let (row, column) = sequence.strip_suffix('H')?.split_once(';')?;
                Some((
                    column.parse::<u16>().ok()?.checked_sub(1)?,
                    row.parse::<u16>().ok()?.checked_sub(1)?,
                ))
            }),
        });
        rest = &rest[end + 2..];
    }
    Ok(commands)
}

fn number(command: &Command, key: &str) -> Option<u64> {
    command.keys.get(key).and_then(|value| value.parse().ok())
}

#[test]
fn a_transmitted_image_carries_the_pixels_the_protocol_expects() -> Result<(), String> {
    let rgba: Vec<u8> = (0..4 * 3 * 4)
        .map(|index| u8::try_from(index).unwrap_or(0))
        .collect();
    let image = PixelImage::new(4, 3, &rgba).ok_or("bad image")?;
    let mut layer = PixelLayer::new(PixelMode::Kitty);
    layer.begin_frame();
    layer.image(5, 1, &image, Rect::new(2, 1, 4, 3), 0);
    let commands = parse(layer.finish_frame())?;
    let transmit = commands.first().ok_or("no transmit command")?;
    if transmit.keys.get("a").map(String::as_str) != Some("t") {
        return Err("first command is not a transmit".to_owned());
    }
    if number(transmit, "f") != Some(32)
        || number(transmit, "s") != Some(4)
        || number(transmit, "v") != Some(3)
    {
        return Err("format or dimensions wrong".to_owned());
    }
    if transmit.payload != rgba {
        return Err("decoded payload does not equal the pixels sent".to_owned());
    }
    let place = commands.get(1).ok_or("no placement command")?;
    if place.keys.get("a").map(String::as_str) != Some("p")
        || number(place, "c") != Some(4)
        || number(place, "r") != Some(3)
    {
        return Err("placement does not cover the image cells".to_owned());
    }
    Ok(())
}

#[test]
fn a_chunked_image_reassembles_and_every_chunk_is_within_the_protocol_limit() -> Result<(), String>
{
    let rgba: Vec<u8> = (0..60 * 40 * 4)
        .map(|index| u8::try_from(index % 253).unwrap_or(0))
        .collect();
    let image = PixelImage::new(60, 40, &rgba).ok_or("bad image")?;
    let mut layer = PixelLayer::new(PixelMode::Kitty);
    layer.begin_frame();
    layer.image(7, 1, &image, Rect::new(0, 0, 10, 5), 0);
    let commands = parse(layer.finish_frame())?;
    let mut joined = Vec::new();
    let transfers: Vec<&Command> = commands
        .iter()
        .take_while(|command| command.keys.get("a").map(String::as_str) != Some("p"))
        .collect();
    for (index, command) in transfers.iter().enumerate() {
        if command.payload.len() > 4096 {
            return Err("chunk exceeds 4096 bytes".to_owned());
        }
        let more = command.keys.get("m").map(String::as_str);
        let expect = if index + 1 == transfers.len() {
            "0"
        } else {
            "1"
        };
        if more != Some(expect) {
            return Err(format!("chunk {index} has m={more:?}, expected m={expect}"));
        }
        joined.extend_from_slice(&command.payload);
    }
    if transfers.len() < 2 {
        return Err("the image was not split into chunks".to_owned());
    }
    if joined != rgba {
        return Err("reassembled pixels differ from the source".to_owned());
    }
    Ok(())
}

#[test]
fn an_unchanged_frame_emits_no_graphics_commands() -> Result<(), String> {
    let rgba = vec![9_u8; 4];
    let image = PixelImage::new(1, 1, &rgba).ok_or("bad image")?;
    let mut layer = PixelLayer::new(PixelMode::Kitty);
    layer.begin_frame();
    layer.image(3, 1, &image, Rect::new(0, 0, 2, 1), 0);
    layer.finish_frame();
    layer.begin_frame();
    layer.image(3, 1, &image, Rect::new(0, 0, 2, 1), 0);
    if !parse(layer.finish_frame())?.is_empty() {
        return Err("an unchanged frame sent graphics commands".to_owned());
    }
    Ok(())
}

#[test]
fn removal_sends_a_delete_for_the_image_id() -> Result<(), String> {
    let rgba = vec![1_u8; 4];
    let image = PixelImage::new(1, 1, &rgba).ok_or("bad image")?;
    let mut layer = PixelLayer::new(PixelMode::Kitty);
    layer.begin_frame();
    layer.image(4, 1, &image, Rect::new(0, 0, 2, 1), 0);
    layer.finish_frame();
    layer.begin_frame();
    let commands = parse(layer.finish_frame())?;
    let delete = commands.first().ok_or("no delete command")?;
    if delete.keys.get("a").map(String::as_str) != Some("d") || number(delete, "i") != Some(4) {
        return Err("removal did not delete image 4".to_owned());
    }
    Ok(())
}

#[test]
fn cells_mode_writes_no_bytes_to_the_capture() {
    let rgba = vec![1_u8; 4];
    let image = PixelImage::new(1, 1, &rgba);
    let mut layer = PixelLayer::new(PixelMode::Cells);
    layer.begin_frame();
    if let Some(image) = image {
        layer.image(1, 1, &image, Rect::new(0, 0, 2, 1), 0);
    }
    assert_eq!(layer.finish_frame(), b"");
}

#[test]
fn moving_and_changing_depth_places_without_transmitting() -> Result<(), String> {
    let rgba = vec![2_u8; 4];
    let image = PixelImage::new(1, 1, &rgba).ok_or("bad image")?;
    let mut layer = PixelLayer::new(PixelMode::Kitty);
    layer.begin_frame();
    layer.image(6, 1, &image, Rect::new(0, 0, 2, 1), 0);
    layer.finish_frame();
    layer.begin_frame();
    layer.image(6, 1, &image, Rect::new(3, 1, 2, 1), -2);
    let commands = parse(layer.finish_frame())?;
    if commands
        .iter()
        .any(|command| command.keys.get("a").map(String::as_str) == Some("t"))
    {
        return Err("a move retransmitted the pixels".to_owned());
    }
    let place = commands.first().ok_or("no placement")?;
    if place.keys.get("a").map(String::as_str) != Some("p") || number(place, "i") != Some(6) {
        return Err(format!("placement is not for image 6: {:?}", place.keys));
    }
    if place.keys.get("z").map(String::as_str) != Some("-2") {
        return Err("depth -2 was not written as a signed value".to_owned());
    }
    Ok(())
}

#[test]
fn clear_deletes_every_image_it_placed() -> Result<(), String> {
    let rgba = vec![4_u8; 4];
    let image = PixelImage::new(1, 1, &rgba).ok_or("bad image")?;
    let mut layer = PixelLayer::new(PixelMode::Kitty);
    layer.begin_frame();
    layer.image(8, 1, &image, Rect::new(0, 0, 2, 1), 0);
    layer.image(9, 1, &image, Rect::new(2, 0, 2, 1), 0);
    layer.finish_frame();
    let commands = parse(layer.clear())?;
    let ids: Vec<u64> = commands
        .iter()
        .filter_map(|command| number(command, "i"))
        .collect();
    if ids != vec![8, 9] {
        return Err(format!("clear deleted {ids:?}, expected [8, 9]"));
    }
    Ok(())
}

#[test]
fn a_changed_generation_is_placed_before_the_old_image_is_deleted() -> Result<(), String> {
    let first = vec![1_u8; 4];
    let second = vec![2_u8; 4];
    let before = PixelImage::new(1, 1, &first).ok_or("bad image")?;
    let after = PixelImage::new(1, 1, &second).ok_or("bad image")?;
    let mut layer = PixelLayer::new(PixelMode::Kitty);
    layer.begin_frame();
    layer.image(10, 1, &before, Rect::new(0, 0, 2, 1), 0);
    layer.finish_frame();
    layer.begin_frame();
    layer.image(10, 2, &after, Rect::new(0, 0, 2, 1), 0);
    let commands = parse(layer.finish_frame())?;
    let actions: Vec<&str> = commands
        .iter()
        .map(|command| command.keys.get("a").map_or("", String::as_str))
        .collect();
    if actions != ["t", "p", "d"] {
        return Err(format!(
            "order was {actions:?}, expected [\"t\", \"p\", \"d\"]"
        ));
    }
    assert_eq!(number(&commands[0], "i"), number(&commands[1], "i"));
    assert_ne!(number(&commands[0], "i"), Some(10));
    assert_eq!(number(&commands[2], "i"), Some(10));
    if commands[0].payload != second {
        return Err("the transmitted pixels are not the new generation".to_owned());
    }
    Ok(())
}

#[derive(Default)]
struct Screen {
    images: BTreeMap<u64, Vec<u8>>,
    visible: BTreeMap<u64, Vec<u8>>,
    transfer: Option<(u64, Vec<u8>)>,
    placements: BTreeMap<u64, (Rect, i32)>,
}

impl Screen {
    fn apply(&mut self, command: &Command) -> Result<(), String> {
        match command.keys.get("a").map(String::as_str) {
            Some("t") => {
                let id = number(command, "i").ok_or("upload without id")?;
                self.images.remove(&id);
                self.visible.remove(&id);
                self.placements.remove(&id);
                self.transfer = Some((id, Vec::new()));
            }
            Some("p") => {
                let id = number(command, "i").ok_or("placement without id")?;
                let pixels = self.images.get(&id).ok_or("placing missing image")?;
                self.visible.insert(id, pixels.clone());
                let (x, y) = command.cursor.ok_or("placement without cursor")?;
                let width = u16::try_from(number(command, "c").ok_or("missing columns")?)
                    .map_err(|error| error.to_string())?;
                let height = u16::try_from(number(command, "r").ok_or("missing rows")?)
                    .map_err(|error| error.to_string())?;
                let depth = command
                    .keys
                    .get("z")
                    .and_then(|value| value.parse().ok())
                    .ok_or("missing depth")?;
                self.placements
                    .insert(id, (Rect::new(x, y, width, height), depth));
            }
            Some("d") => {
                let id = number(command, "i").ok_or("delete without id")?;
                self.images.remove(&id);
                self.visible.remove(&id);
                self.placements.remove(&id);
            }
            _ => {}
        }
        if !command.payload.is_empty() {
            let (_, pixels) = self.transfer.as_mut().ok_or("chunk without upload")?;
            pixels.extend_from_slice(&command.payload);
            if number(command, "m") != Some(1) {
                let (id, pixels) = self.transfer.take().ok_or("missing upload")?;
                self.images.insert(id, pixels);
            }
        }
        Ok(())
    }

    fn at(&self, area: Rect, depth: i32) -> Option<&[u8]> {
        self.placements.iter().find_map(|(id, placement)| {
            (*placement == (area, depth))
                .then(|| self.visible.get(id).map(Vec::as_slice))
                .flatten()
        })
    }
}

#[test]
fn every_upload_chunk_keeps_the_previous_image_visible_until_replacement() -> Result<(), String> {
    let first = [255, 0, 0, 255];
    let second = [0, 255, 0, 255].repeat(2048);
    let before = PixelImage::new(1, 1, &first).ok_or("bad image")?;
    let after = PixelImage::new(2048, 1, &second).ok_or("bad image")?;
    let mut layer = PixelLayer::new(PixelMode::Kitty);
    let mut screen = Screen::default();
    layer.begin_frame();
    layer.image(10, 1, &before, Rect::new(0, 0, 2, 1), 0);
    for command in parse(layer.finish_frame())? {
        screen.apply(&command)?;
    }
    assert_eq!(
        screen.visible.values().collect::<Vec<_>>(),
        vec![&first.to_vec()]
    );
    for generation in 2..5 {
        layer.begin_frame();
        layer.image(10, generation, &after, Rect::new(0, 0, 2, 1), 0);
        for command in parse(layer.finish_frame())? {
            screen.apply(&command)?;
            assert!(
                !screen.visible.is_empty(),
                "image vanished during {command:?}"
            );
        }
        assert_eq!(screen.visible.values().collect::<Vec<_>>(), vec![&second]);
        assert_eq!(screen.images.len(), 1, "old uploads must be freed");
    }
    for command in parse(layer.clear())? {
        screen.apply(&command)?;
    }
    assert!(screen.visible.is_empty());
    assert!(screen.images.is_empty());
    Ok(())
}

#[test]
fn fit_changes_keep_pixels_visible_and_wire_ids_do_not_collide_with_widget_ids()
-> Result<(), Box<dyn std::error::Error>> {
    let source = ImageSource::rgba8(
        2,
        2,
        vec![10, 0, 0, 255, 20, 0, 0, 255, 30, 0, 0, 255, 40, 0, 0, 255].into_boxed_slice(),
    )?;
    let mut layer = PixelLayer::new(PixelMode::Kitty);
    let mut screen = Screen::default();
    for fit in [Fit::Stretch, Fit::Cover, Fit::Stretch, Fit::Cover] {
        layer.begin_frame();
        assert!(layer.submit(ImageRequest::new(
            ImageId::new(10).ok_or("invalid id")?,
            1,
            &source,
            Rect::new(0, 0, 1, 1),
            fit,
            0,
            255,
        )));
        let was_visible = !screen.visible.is_empty();
        for command in parse(layer.finish_frame())? {
            screen.apply(&command)?;
            if was_visible {
                assert!(
                    !screen.visible.is_empty(),
                    "image vanished during {command:?}"
                );
            }
        }
        let expected = if fit == Fit::Cover {
            vec![10, 0, 0, 255, 30, 0, 0, 255]
        } else {
            vec![10, 0, 0, 255, 20, 0, 0, 255, 30, 0, 0, 255, 40, 0, 0, 255]
        };
        assert_eq!(screen.visible.values().collect::<Vec<_>>(), vec![&expected]);
    }
    let pixel = PixelImage::new(1, 1, &[0, 255, 0, 255]).ok_or("bad pixel")?;
    layer.begin_frame();
    layer.image(10, 2, &pixel, Rect::new(0, 0, 1, 1), 0);
    layer.image(11, 1, &pixel, Rect::new(1, 0, 1, 1), 0);
    for command in parse(layer.finish_frame())? {
        screen.apply(&command)?;
        assert!(!screen.visible.is_empty());
    }
    assert_eq!(
        screen.visible.values().collect::<Vec<_>>(),
        vec![&vec![0, 255, 0, 255], &vec![0, 255, 0, 255]]
    );
    assert_eq!(screen.images.len(), 2);
    for command in parse(layer.clear())? {
        screen.apply(&command)?;
    }
    assert!(screen.images.is_empty());
    assert!(screen.visible.is_empty());
    Ok(())
}

fn measured_layer() -> PixelLayer {
    PixelLayer::from_caps(&Caps {
        protocols: Protocols {
            graphics: GraphicsProtocol::Kitty,
            ..Protocols::default()
        },
        cell_pixels: CellPixels::new(1, 1),
        ..Caps::default()
    })
}

fn solid_source(width: u32, height: u32, rgba: [u8; 4]) -> Result<ImageSource, String> {
    ImageSource::rgba8(
        width,
        height,
        rgba.repeat(usize::try_from(width * height).map_err(|error| error.to_string())?)
            .into_boxed_slice(),
    )
    .map_err(|error| error.to_string())
}

#[test]
fn automatic_rasters_cache_exact_pixels_and_move_without_losing_the_display() -> Result<(), String>
{
    let mut layer = measured_layer();
    let red = solid_source(1024, 2, [255, 0, 0, 255])?;
    let green = solid_source(1024, 2, [0, 255, 0, 255])?;
    let area = Rect::new(0, 0, 1024, 2);
    let moved = Rect::new(4, 5, 1024, 2);
    let mut screen = Screen::default();
    layer.begin_frame();
    assert!(layer.submit_raster(&red, area, 0));
    for command in parse(layer.finish_frame())? {
        screen.apply(&command)?;
    }
    assert_eq!(
        screen.at(area, 0),
        Some([255, 0, 0, 255].repeat(2048).as_slice())
    );
    layer.begin_frame();
    let equal_but_distinct_source = solid_source(1024, 2, [255, 0, 0, 255])?;
    assert!(layer.submit_raster(&equal_but_distinct_source, area, 0));
    assert_eq!(layer.finish_frame(), b"");
    layer.begin_frame();
    assert!(layer.submit_raster(&red, moved, -1));
    let movement = parse(layer.finish_frame())?;
    assert_eq!(movement.len(), 1);
    for command in movement {
        screen.apply(&command)?;
    }
    assert_eq!(screen.at(moved, -1), Some(red.rgba8_pixels()));
    layer.begin_frame();
    assert!(layer.submit_raster(&green, moved, -1));
    for command in parse(layer.finish_frame())? {
        screen.apply(&command)?;
        assert!(screen.at(moved, -1).is_some(), "blank during {command:?}");
    }
    assert_eq!(
        screen.at(moved, -1),
        Some([0, 255, 0, 255].repeat(2048).as_slice())
    );
    assert_eq!(screen.images.len(), 1);
    for command in parse(layer.clear())? {
        screen.apply(&command)?;
    }
    assert!(screen.images.is_empty());
    assert!(screen.visible.is_empty());
    layer.begin_frame();
    assert!(layer.submit_raster(&green, moved, -1));
    for command in parse(layer.finish_frame())? {
        screen.apply(&command)?;
    }
    assert_eq!(screen.at(moved, -1), Some(green.rgba8_pixels()));
    Ok(())
}

#[test]
fn automatic_slot_reorders_and_caller_id_extremes_keep_independent_pixels() -> Result<(), String> {
    let mut layer = measured_layer();
    let red = solid_source(1, 1, [255, 0, 0, 255])?;
    let blue = solid_source(1, 1, [0, 0, 255, 255])?;
    let white = PixelImage::new(1, 1, &[255, 255, 255, 255]).ok_or("bad white")?;
    let (left, right, caller_one, caller_max) = (
        Rect::new(0, 0, 1, 1),
        Rect::new(1, 0, 1, 1),
        Rect::new(2, 0, 1, 1),
        Rect::new(3, 0, 1, 1),
    );
    let mut screen = Screen::default();
    for reverse in [false, true, false] {
        layer.begin_frame();
        let sources = if reverse {
            [&blue, &red]
        } else {
            [&red, &blue]
        };
        assert!(layer.submit_raster(sources[0], left, 0));
        layer.image(1, 0, &white, caller_one, 0);
        assert!(layer.submit_raster(sources[1], right, -1));
        layer.image(u32::MAX, 0, &white, caller_max, 0);
        for command in parse(layer.finish_frame())? {
            screen.apply(&command)?;
        }
        assert_eq!(screen.at(left, 0), Some(sources[0].rgba8_pixels()));
        assert_eq!(screen.at(right, -1), Some(sources[1].rgba8_pixels()));
        assert_eq!(
            screen.at(caller_one, 0),
            Some([255, 255, 255, 255].as_slice())
        );
        assert_eq!(
            screen.at(caller_max, 0),
            Some([255, 255, 255, 255].as_slice())
        );
        assert_eq!(screen.images.len(), 4);
    }
    layer.begin_frame();
    assert!(layer.submit_raster(&blue, right, -1));
    for command in parse(layer.finish_frame())? {
        screen.apply(&command)?;
    }
    assert_eq!(screen.at(right, -1), Some([0, 0, 255, 255].as_slice()));
    assert_eq!(screen.images.len(), 1);
    layer.begin_frame();
    for command in parse(layer.finish_frame())? {
        screen.apply(&command)?;
    }
    assert!(screen.images.is_empty());
    layer.begin_frame();
    assert!(layer.submit_raster(&red, left, 0));
    assert!(layer.submit_raster(&blue, right, -1));
    for command in parse(layer.finish_frame())? {
        screen.apply(&command)?;
    }
    assert_eq!(screen.at(left, 0), Some([255, 0, 0, 255].as_slice()));
    assert_eq!(screen.at(right, -1), Some([0, 0, 255, 255].as_slice()));
    Ok(())
}

#[test]
fn equal_raster_pixels_can_reorder_placements_and_depth_without_uploads() -> Result<(), String> {
    let mut layer = measured_layer();
    let red = solid_source(1, 1, [255, 0, 0, 255])?;
    let left = Rect::new(0, 0, 1, 1);
    let right = Rect::new(3, 2, 1, 1);
    let mut screen = Screen::default();
    layer.begin_frame();
    assert!(layer.submit_raster(&red, left, -1));
    assert!(layer.submit_raster(&red, right, 0));
    for command in parse(layer.finish_frame())? {
        screen.apply(&command)?;
    }
    layer.begin_frame();
    assert!(layer.submit_raster(&red, right, 2));
    assert!(layer.submit_raster(&red, left, 1));
    let commands = parse(layer.finish_frame())?;
    assert_eq!(commands.len(), 2);
    for command in commands {
        assert_eq!(command.keys.get("a").map(String::as_str), Some("p"));
        screen.apply(&command)?;
    }
    assert_eq!(screen.at(right, 2), Some([255, 0, 0, 255].as_slice()));
    assert_eq!(screen.at(left, 1), Some([255, 0, 0, 255].as_slice()));
    assert_eq!(screen.images.len(), 2);
    Ok(())
}

#[test]
fn raster_rejection_does_not_consume_an_accepted_slot() -> Result<(), String> {
    let mut layer = measured_layer();
    let red = solid_source(1, 1, [255, 0, 0, 255])?;
    let blue = solid_source(1, 1, [0, 0, 255, 255])?;
    let mismatched = solid_source(2, 1, [0, 255, 0, 255])?;
    let long = solid_source(4097, 1, [0, 255, 0, 255])?;
    let oversized = solid_source(1025, 1024, [0, 255, 0, 255])?;
    let area = Rect::new(0, 0, 1, 1);
    layer.begin_frame();
    assert!(layer.submit_raster(&red, area, 0));
    assert!(!layer.submit_raster(&mismatched, area, 0));
    assert!(!layer.submit_raster(&long, Rect::new(0, 0, 4097, 1), 0));
    assert!(!layer.submit_raster(&oversized, Rect::new(0, 0, 1025, 1024), 0));
    assert!(layer.submit_raster(&blue, Rect::new(1, 0, 1, 1), 0));
    let mut screen = Screen::default();
    for command in parse(layer.finish_frame())? {
        screen.apply(&command)?;
    }
    assert_eq!(screen.at(area, 0), Some([255, 0, 0, 255].as_slice()));
    assert_eq!(
        screen.at(Rect::new(1, 0, 1, 1), 0),
        Some([0, 0, 255, 255].as_slice())
    );
    layer.begin_frame();
    assert!(layer.submit_raster(&red, area, 0));
    assert!(layer.submit_raster(&blue, Rect::new(1, 0, 1, 1), 0));
    assert_eq!(layer.finish_frame(), b"");
    let mut no_metrics = PixelLayer::new(PixelMode::Kitty);
    no_metrics.begin_frame();
    assert!(!no_metrics.submit_raster(&red, area, 0));
    assert_eq!(no_metrics.finish_frame(), b"");
    Ok(())
}

#[test]
fn maximum_raster_count_does_not_wrap_and_omitted_slots_are_forgotten() -> Result<(), String> {
    let mut layer = measured_layer();
    let red = solid_source(1, 1, [255, 0, 0, 255])?;
    let mut screen = Screen::default();
    layer.begin_frame();
    for x in 0..256 {
        assert!(layer.submit_raster(&red, Rect::new(x, 0, 1, 1), 0));
    }
    assert!(!layer.submit_raster(&red, Rect::new(256, 0, 1, 1), 0));
    for command in parse(layer.finish_frame())? {
        screen.apply(&command)?;
    }
    assert_eq!(screen.images.len(), 256);
    assert_eq!(
        screen.at(Rect::new(255, 0, 1, 1), 0),
        Some([255, 0, 0, 255].as_slice())
    );
    layer.begin_frame();
    for x in 0..256 {
        assert!(layer.submit_raster(&red, Rect::new(x, 0, 1, 1), 0));
    }
    assert_eq!(layer.finish_frame(), b"");
    layer.begin_frame();
    for command in parse(layer.finish_frame())? {
        screen.apply(&command)?;
    }
    assert!(screen.images.is_empty());
    assert!(screen.visible.is_empty());
    layer.begin_frame();
    assert!(layer.submit_raster(&red, Rect::new(0, 0, 1, 1), 0));
    for command in parse(layer.finish_frame())? {
        screen.apply(&command)?;
    }
    assert_eq!(
        screen.at(Rect::new(0, 0, 1, 1), 0),
        Some([255, 0, 0, 255].as_slice())
    );
    Ok(())
}

#[test]
fn automatic_rasters_share_the_accepted_frame_pixel_budget() -> Result<(), String> {
    let mut layer = measured_layer();
    let large = solid_source(1024, 1024, [255, 0, 0, 255])?;
    let small = solid_source(1, 1, [0, 0, 255, 255])?;
    layer.begin_frame();
    let caller_image =
        PixelImage::new(1024, 1024, large.rgba8_pixels()).ok_or("bad caller image")?;
    layer.image(99, 0, &caller_image, Rect::new(0, 0, 1024, 1024), 0);
    for x in 1..4 {
        assert!(layer.submit_raster(&large, Rect::new(x * 1024, 0, 1024, 1024), 0));
    }
    assert!(!layer.submit_raster(&small, Rect::new(0, 0, 1, 1), 0));
    let output = layer.finish_frame();
    assert_eq!(
        output.windows(7).filter(|part| *part == b"_Ga=t,i").count(),
        4
    );
    layer.begin_frame();
    assert!(layer.submit_raster(&small, Rect::new(0, 0, 1, 1), 0));
    let mut screen = Screen::default();
    for command in parse(layer.finish_frame())? {
        screen.apply(&command)?;
    }
    assert_eq!(
        screen.at(Rect::new(0, 0, 1, 1), 0),
        Some([0, 0, 255, 255].as_slice())
    );
    Ok(())
}
