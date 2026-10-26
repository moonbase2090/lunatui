// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

//! Headless protocol capture: the bytes `PixelLayer` emits are parsed as kitty graphics
//! APC commands (`ESC _ G <control-keys> ; <base64 payload> ESC \`) and checked against the
//! protocol rules, independent of the layer's own writer.

#![cfg(feature = "pixel-layer")]

use std::collections::BTreeMap;

use lunatui_core::Rect;
use lunatui_term::{PixelImage, PixelLayer, PixelMode};

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// One parsed APC command.
#[derive(Debug)]
struct Command {
    keys: BTreeMap<String, String>,
    payload: Vec<u8>,
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

/// Splits a byte stream into APC graphics commands; anything else must be cursor moves.
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
    let mut seen_last = false;
    for command in commands
        .iter()
        .take_while(|command| command.keys.get("a").map(String::as_str) != Some("p"))
    {
        if command.payload.len() > 4096 {
            return Err("chunk exceeds 4096 bytes".to_owned());
        }
        joined.extend_from_slice(&command.payload);
        seen_last = command.keys.get("m").map(String::as_str) == Some("0");
    }
    if !seen_last {
        return Err("the last chunk does not end the transfer".to_owned());
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
