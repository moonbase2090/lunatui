// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::Rect;
use lunatui_term::{
    Caps, GraphicsProtocol, Multiplexer, PixelImage, PixelLayer, PixelMode, Protocols, TermIdentity,
};

const PIXEL: [u8; 4] = [1, 2, 3, 4];

fn frame(layer: &mut PixelLayer, build: impl FnOnce(&mut PixelLayer)) -> Vec<u8> {
    layer.begin_frame();
    build(layer);
    layer.finish_frame().to_vec()
}

fn one(layer: &mut PixelLayer, id: u32, generation: u64, rect: Rect, z: i32) -> Vec<u8> {
    let image = PixelImage::new(1, 1, &PIXEL);
    frame(layer, |layer| {
        if let Some(image) = image {
            layer.image(id, generation, &image, rect, z);
        }
    })
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn decode(payload: &str) -> Vec<u8> {
    const ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let mut bits = 0_u32;
    let mut count = 0;
    for ch in payload.chars().filter(|ch| *ch != '=') {
        let value = ALPHABET
            .find(ch)
            .map_or(0, |index| u32::try_from(index).unwrap_or(0));
        bits = bits << 6 | value;
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push(u8::try_from((bits >> count) & 0xff).unwrap_or(0));
        }
    }
    out
}

#[test]
fn pixel_image_rejects_empty_sides_and_wrong_lengths() {
    assert!(PixelImage::new(1, 1, &PIXEL).is_some());
    assert!(PixelImage::new(0, 1, &[]).is_none());
    assert!(PixelImage::new(1, 0, &[]).is_none());
    assert!(PixelImage::new(2, 1, &PIXEL).is_none());
    assert!(PixelImage::new(1, 1, &[1, 2, 3]).is_none());
}

#[test]
fn mode_needs_the_kitty_reply_and_no_multiplexer() {
    let kitty = Protocols {
        graphics: GraphicsProtocol::Kitty,
        ..Protocols::default()
    };
    let direct = Caps {
        protocols: kitty,
        ..Caps::default()
    };
    assert_eq!(PixelMode::from_caps(&direct), PixelMode::Kitty);
    assert_eq!(PixelMode::from_caps(&Caps::default()), PixelMode::Cells);
    for multiplexer in [Multiplexer::Tmux, Multiplexer::Screen, Multiplexer::Both] {
        let wrapped = Caps {
            protocols: kitty,
            identity: TermIdentity {
                multiplexer,
                ..TermIdentity::default()
            },
            ..Caps::default()
        };
        assert_eq!(
            PixelMode::from_caps(&wrapped),
            PixelMode::Cells,
            "{multiplexer:?}"
        );
    }
    assert!(PixelMode::Kitty.is_pixels());
    assert!(!PixelMode::Cells.is_pixels());
}

#[test]
fn cells_mode_emits_nothing() {
    let mut layer = PixelLayer::new(PixelMode::Cells);
    assert_eq!(
        one(&mut layer, 7, 1, Rect::new(0, 0, 4, 2), 0),
        Vec::<u8>::new()
    );
    assert_eq!(layer.clear(), b"");
}

#[test]
fn a_new_image_is_transmitted_then_placed() {
    let mut layer = PixelLayer::new(PixelMode::Kitty);
    let bytes = one(&mut layer, 7, 1, Rect::new(3, 2, 10, 4), -1);
    assert_eq!(
        text(&bytes),
        "\x1b_Ga=t,i=7,f=32,s=1,v=1,q=2;AQIDBA==\x1b\\\
         \x1b[3;4H\x1b_Ga=p,i=7,p=1,c=10,r=4,z=-1,C=1,q=2\x1b\\"
    );
}

#[test]
fn an_unchanged_image_costs_nothing() {
    let mut layer = PixelLayer::new(PixelMode::Kitty);
    one(&mut layer, 7, 1, Rect::new(3, 2, 10, 4), 0);
    assert_eq!(
        one(&mut layer, 7, 1, Rect::new(3, 2, 10, 4), 0),
        Vec::<u8>::new()
    );
    assert_eq!(
        one(&mut layer, 7, 1, Rect::new(3, 2, 10, 4), 0),
        Vec::<u8>::new()
    );
}

#[test]
fn moving_or_changing_depth_places_again_without_transmitting() {
    let mut layer = PixelLayer::new(PixelMode::Kitty);
    one(&mut layer, 7, 1, Rect::new(3, 2, 10, 4), 0);
    let moved = one(&mut layer, 7, 1, Rect::new(5, 2, 10, 4), 0);
    assert_eq!(
        text(&moved),
        "\x1b[3;6H\x1b_Ga=p,i=7,p=1,c=10,r=4,z=0,C=1,q=2\x1b\\"
    );
    let deeper = one(&mut layer, 7, 1, Rect::new(5, 2, 10, 4), 3);
    assert_eq!(
        text(&deeper),
        "\x1b[3;6H\x1b_Ga=p,i=7,p=1,c=10,r=4,z=3,C=1,q=2\x1b\\"
    );
    let resized = one(&mut layer, 7, 1, Rect::new(5, 2, 12, 4), 3);
    assert!(text(&resized).contains("c=12,r=4"));
    assert!(!text(&resized).contains("a=t"));
}

#[test]
fn new_content_is_transmitted_and_placed_again() {
    let mut layer = PixelLayer::new(PixelMode::Kitty);
    one(&mut layer, 7, 1, Rect::new(0, 0, 4, 2), 0);
    let bytes = text(&one(&mut layer, 7, 2, Rect::new(0, 0, 4, 2), 0));
    assert!(bytes.starts_with("\x1b_Ga=t,i=7,"));
    assert!(bytes.contains("a=p,i=7,"));
}

#[test]
fn a_changed_size_with_the_same_generation_is_transmitted_again() {
    let mut layer = PixelLayer::new(PixelMode::Kitty);
    one(&mut layer, 7, 1, Rect::new(0, 0, 4, 2), 0);
    let two = [9_u8; 8];
    let wide = PixelImage::new(2, 1, &two);
    let bytes = frame(&mut layer, |layer| {
        if let Some(image) = wide {
            layer.image(7, 1, &image, Rect::new(0, 0, 4, 2), 0);
        }
    });
    assert!(text(&bytes).starts_with("\x1b_Ga=t,i=7,f=32,s=2,v=1,"));
}

#[test]
fn an_image_that_is_no_longer_listed_is_deleted_and_forgotten() {
    let mut layer = PixelLayer::new(PixelMode::Kitty);
    one(&mut layer, 7, 1, Rect::new(0, 0, 4, 2), 0);
    let gone = frame(&mut layer, |_| {});
    assert_eq!(text(&gone), "\x1b_Ga=d,d=I,i=7,q=2\x1b\\");
    assert_eq!(frame(&mut layer, |_| {}), Vec::<u8>::new());
    let back = text(&one(&mut layer, 7, 1, Rect::new(0, 0, 4, 2), 0));
    assert!(back.starts_with("\x1b_Ga=t,i=7,"));
}

#[test]
fn clear_deletes_everything_and_resets() {
    let mut layer = PixelLayer::new(PixelMode::Kitty);
    let image = PixelImage::new(1, 1, &PIXEL);
    frame(&mut layer, |layer| {
        if let Some(image) = image {
            layer.image(1, 1, &image, Rect::new(0, 0, 2, 1), 0);
            layer.image(2, 1, &image, Rect::new(2, 0, 2, 1), 0);
        }
    });
    let cleared = text(layer.clear());
    assert_eq!(
        cleared,
        "\x1b_Ga=d,d=I,i=1,q=2\x1b\\\x1b_Ga=d,d=I,i=2,q=2\x1b\\"
    );
    assert_eq!(layer.clear(), b"");
    assert!(text(&one(&mut layer, 1, 1, Rect::new(0, 0, 2, 1), 0)).starts_with("\x1b_Ga=t,i=1,"));
}

#[test]
fn zero_ids_empty_rects_and_repeated_ids_are_ignored() {
    let mut layer = PixelLayer::new(PixelMode::Kitty);
    assert_eq!(
        one(&mut layer, 0, 1, Rect::new(0, 0, 2, 1), 0),
        Vec::<u8>::new()
    );
    assert_eq!(
        one(&mut layer, 4, 1, Rect::new(0, 0, 0, 1), 0),
        Vec::<u8>::new()
    );
    assert_eq!(
        one(&mut layer, 4, 1, Rect::new(0, 0, 2, 0), 0),
        Vec::<u8>::new()
    );
    let image = PixelImage::new(1, 1, &PIXEL);
    let bytes = frame(&mut layer, |layer| {
        if let Some(image) = image {
            layer.image(5, 1, &image, Rect::new(0, 0, 2, 1), 0);
            layer.image(5, 9, &image, Rect::new(7, 7, 3, 3), 1);
        }
    });
    let bytes = text(&bytes);
    assert_eq!(bytes.matches("a=t").count(), 1);
    assert!(bytes.contains("c=2,r=1,z=0"));
}

#[test]
fn a_large_image_is_split_into_chunks_that_decode_to_the_pixels() {
    let rgba: Vec<u8> = (0..40 * 20 * 4)
        .map(|index| u8::try_from(index % 251).unwrap_or(0))
        .collect();
    let image = PixelImage::new(40, 20, &rgba);
    let mut layer = PixelLayer::new(PixelMode::Kitty);
    let bytes = text(&frame(&mut layer, |layer| {
        if let Some(image) = image {
            layer.image(9, 1, &image, Rect::new(0, 0, 5, 2), 0);
        }
    }));
    let commands: Vec<&str> = bytes
        .split("\x1b\\")
        .filter(|part| !part.is_empty())
        .collect();
    assert_eq!(commands.len(), 3);
    assert!(commands[0].starts_with("\x1b_Ga=t,i=9,f=32,s=40,v=20,q=2,m=1;"));
    assert!(commands[1].starts_with("\x1b_Gm=0;"));
    assert!(commands[2].contains("a=p,i=9"));
    let first = commands[0]
        .split_once(';')
        .map_or("", |(_, payload)| payload);
    let second = commands[1]
        .split_once(';')
        .map_or("", |(_, payload)| payload);
    assert_eq!(first.len(), 4096);
    let mut decoded = decode(first);
    decoded.extend(decode(second));
    assert_eq!(decoded, rgba);
}

#[test]
fn three_chunks_mark_the_middle_one_as_more_to_come() {
    let rgba = vec![5_u8; 1537 * 4];
    let image = PixelImage::new(1537, 1, &rgba);
    let mut layer = PixelLayer::new(PixelMode::Kitty);
    let bytes = text(&frame(&mut layer, |layer| {
        if let Some(image) = image {
            layer.image(2, 1, &image, Rect::new(0, 0, 1, 1), 0);
        }
    }));
    let commands: Vec<&str> = bytes
        .split("\x1b\\")
        .filter(|part| !part.is_empty())
        .collect();
    assert_eq!(commands.len(), 4);
    assert!(commands[0].contains(",q=2,m=1;"));
    assert!(commands[1].starts_with("\x1b_Gm=1;"));
    assert!(commands[2].starts_with("\x1b_Gm=0;"));
}

#[test]
fn depth_is_written_signed() {
    let mut layer = PixelLayer::new(PixelMode::Kitty);
    assert!(
        text(&one(&mut layer, 1, 1, Rect::new(0, 0, 1, 1), i32::MIN)).contains("z=-2147483648,")
    );
    assert!(text(&one(&mut layer, 1, 1, Rect::new(0, 0, 1, 1), 0)).contains("z=0,"));
}
