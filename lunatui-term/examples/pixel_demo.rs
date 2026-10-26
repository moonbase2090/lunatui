// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

//! Draws a gradient through the kitty graphics protocol using `PixelLayer`, then waits so the
//! terminal keeps showing it. Run it inside a kitty-graphics terminal such as Ghostty.

use std::io::{self, Write};
use std::thread;
use std::time::Duration;

use lunatui_core::Rect;
use lunatui_term::{PixelImage, PixelLayer, PixelMode};

const WIDTH: u32 = 64;
const HEIGHT: u32 = 32;

fn main() -> io::Result<()> {
    let mut rgba = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            rgba.extend_from_slice(&[(x * 4) as u8, (y * 8) as u8, 200, 255]);
        }
    }
    let image =
        PixelImage::new(WIDTH, HEIGHT, &rgba).ok_or_else(|| io::Error::other("bad image"))?;
    let mut layer = PixelLayer::new(PixelMode::Kitty);
    layer.begin_frame();
    layer.image(1, 1, &image, Rect::new(4, 2, 16, 8), 0);
    let mut stdout = io::stdout().lock();
    stdout.write_all(b"\x1b[2J\x1b[H")?;
    stdout.write_all(layer.finish_frame())?;
    stdout.write_all(b"\x1b[1;1Hpixel layer: kitty graphics (Ghostty)")?;
    stdout.flush()?;
    thread::sleep(Duration::from_secs(60));
    stdout.write_all(layer.clear())?;
    stdout.flush()
}
