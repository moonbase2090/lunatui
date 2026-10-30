// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::io::Cursor;

use lunatui_core::Rect;
use lunatui_image::{ImageSink, ImageSource};

use super::{CellPixels, ImageKey, PixelLayer, PixelMode};

fn decoded_inline_images(output: &[u8]) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
    let mut images = Vec::new();
    for image in String::from_utf8_lossy(output)
        .split("\x1b]1337;File=")
        .skip(1)
    {
        let (_, encoded) = image.split_once(':').ok_or("missing PNG payload")?;
        let encoded = encoded
            .split_once('\x1b')
            .ok_or("missing image terminator")?
            .0;
        let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut bytes = Vec::new();
        let (mut bits, mut count) = (0_u32, 0_u32);
        for byte in encoded.bytes().take_while(|byte| *byte != b'=') {
            let value = alphabet
                .iter()
                .position(|candidate| *candidate == byte)
                .ok_or("bad base64")?;
            bits = bits << 6 | u32::try_from(value)?;
            count += 6;
            if count >= 8 {
                count -= 8;
                bytes.push(u8::try_from((bits >> count) & 255)?);
            }
        }
        let mut decoder = png::Decoder::new(Cursor::new(bytes)).read_info()?;
        let mut rgba = vec![0; decoder.output_buffer_size().ok_or("unbounded PNG")?];
        let info = decoder.next_frame(&mut rgba)?;
        rgba.truncate(info.buffer_size());
        images.push(rgba);
    }
    Ok(images)
}

#[test]
fn late_inline_byte_limit_rejection_preserves_slots_budget_and_next_frames()
-> Result<(), Box<dyn std::error::Error>> {
    let mut layer = PixelLayer::new(PixelMode::Iterm2);
    layer.cell_pixels = CellPixels::new(1, 1);
    let red = ImageSource::rgba8(1, 1, vec![255, 0, 0, 255].into_boxed_slice())?;
    let blue = ImageSource::rgba8(1, 1, vec![0, 0, 255, 255].into_boxed_slice())?;
    let mut random = 1_u32;
    let noise: Vec<u8> = (0..40_000)
        .map(|_| {
            random = random.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            random.to_be_bytes()[0]
        })
        .collect();
    let noise = ImageSource::rgba8(100, 100, noise.into_boxed_slice())?;
    let left = Rect::new(0, 0, 1, 1);
    let right = Rect::new(1, 0, 1, 1);
    layer.begin_frame();
    assert!(layer.submit_raster(&red, left, 0));
    assert_eq!(
        decoded_inline_images(&layer.out)?,
        vec![vec![255, 0, 0, 255]]
    );
    layer.out.resize(32 * 1024 * 1024 - 1024, 0);
    let before = layer.out.len();
    assert!(!layer.submit_raster(&noise, Rect::new(0, 0, 100, 100), 0));
    assert_eq!(layer.out.len(), before);
    assert_eq!(
        (layer.raster_cursor, layer.frame.len(), layer.frame_pixels),
        (1, 1, 1)
    );
    assert!(layer.uploaded.is_empty());
    assert_eq!(layer.retired, Vec::<u32>::new());
    assert!(layer.submit_raster(&blue, right, 0));
    assert_eq!(
        decoded_inline_images(&layer.out[before..])?,
        vec![vec![0, 0, 255, 255]]
    );
    assert_eq!(layer.frame[0].id, ImageKey::RasterSlot(0));
    assert_eq!(layer.frame[1].id, ImageKey::RasterSlot(1));
    let _output = layer.finish_frame();
    layer.begin_frame();
    assert!(layer.submit_raster(&red, left, 0));
    assert!(layer.submit_raster(&blue, right, 0));
    assert_eq!(
        decoded_inline_images(layer.finish_frame())?,
        vec![vec![255, 0, 0, 255], vec![0, 0, 255, 255]]
    );
    assert_eq!(
        layer
            .raster_slots
            .iter()
            .map(|slot| slot.generation)
            .collect::<Vec<_>>(),
        vec![0, 0]
    );
    layer.begin_frame();
    assert!(!layer.submit_raster(&red, left, -1));
    let _output = layer.finish_frame();
    assert!(layer.raster_slots.is_empty());
    assert_eq!(layer.placed, Vec::<super::Placed>::new());
    Ok(())
}
