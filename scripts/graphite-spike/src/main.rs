// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::collections::HashMap;
use std::time::{Duration, Instant};

use base64::Engine;
use fontdue::{Font, FontSettings};
use sha2::{Digest, Sha256};
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Rect, Transform};

struct Glyphs {
    font: Font,
    cache: HashMap<(char, u32), (fontdue::Metrics, Vec<u8>)>,
    hits: u64,
    misses: u64,
}

impl Glyphs {
    fn new(path: &str) -> Self {
        let bytes = std::fs::read(path).expect("font");
        let font = Font::from_bytes(bytes, FontSettings::default()).expect("parse");
        Self { font, cache: HashMap::new(), hits: 0, misses: 0 }
    }
    fn glyph(&mut self, c: char, px: f32, cached: bool) -> (fontdue::Metrics, Vec<u8>) {
        let key = (c, px.to_bits());
        if cached {
            if let Some(found) = self.cache.get(&key) {
                self.hits += 1;
                return found.clone();
            }
        }
        self.misses += 1;
        let made = self.font.rasterize(c, px);
        if cached {
            self.cache.insert(key, made.clone());
        }
        made
    }
}

fn paint(r: u8, g: u8, b: u8, a: u8) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(r, g, b, a);
    p.anti_alias = true;
    p
}

fn rrect(pix: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, radius: f32, color: (u8, u8, u8, u8)) {
    let r = radius.min(w / 2.0).min(h / 2.0);
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    if let Some(path) = pb.finish() {
        pix.fill_path(&path, &paint(color.0, color.1, color.2, color.3), FillRule::Winding, Transform::identity(), None);
    }
}

fn rect(pix: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, color: (u8, u8, u8, u8)) {
    if let Some(r) = Rect::from_xywh(x, y, w, h) {
        pix.fill_rect(r, &paint(color.0, color.1, color.2, color.3), Transform::identity(), None);
    }
}

fn text(pix: &mut Pixmap, glyphs: &mut Glyphs, cached: bool, x: f32, baseline: f32, px: f32, s: &str, color: (u8, u8, u8)) -> f32 {
    let mut pen = x;
    let width = pix.width() as i32;
    let height = pix.height() as i32;
    let data = pix.data_mut();
    for c in s.chars() {
        let (m, bitmap) = glyphs.glyph(c, px, cached);
        let gx = pen.round() as i32 + m.xmin;
        let gy = baseline.round() as i32 - m.height as i32 - m.ymin;
        for row in 0..m.height {
            let y = gy + row as i32;
            if y < 0 || y >= height {
                continue;
            }
            for col in 0..m.width {
                let xx = gx + col as i32;
                if xx < 0 || xx >= width {
                    continue;
                }
                let cov = bitmap[row * m.width + col] as u32;
                if cov == 0 {
                    continue;
                }
                let i = ((y * width + xx) * 4) as usize;
                let inv = 255 - cov;
                data[i] = ((color.0 as u32 * cov + data[i] as u32 * inv) / 255) as u8;
                data[i + 1] = ((color.1 as u32 * cov + data[i + 1] as u32 * inv) / 255) as u8;
                data[i + 2] = ((color.2 as u32 * cov + data[i + 2] as u32 * inv) / 255) as u8;
                data[i + 3] = 255;
            }
        }
        pen += m.advance_width;
    }
    pen - x
}

const TEXT: (u8, u8, u8) = (0xe6, 0xe9, 0xee);
const MUTED: (u8, u8, u8) = (0xa3, 0xab, 0xb8);
const GREEN: (u8, u8, u8, u8) = (0x4c, 0xc9, 0x8a, 255);
const AMBER: (u8, u8, u8, u8) = (0xf2, 0xb8, 0x4b, 255);
const RED: (u8, u8, u8, u8) = (0xff, 0x7a, 0x6b, 255);

fn lane(pix: &mut Pixmap, s: f32, x: f32, y: f32, w: f32, h: f32, bins: usize, seed: u32) {
    let gap = 1.5 * s;
    let bw = (w - gap * (bins as f32 - 1.0)) / bins as f32;
    let mut state = seed;
    let mut level = 0.5f32;
    for i in 0..bins {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        level = (level + ((state >> 16) as f32 / 65535.0 - 0.5) * 0.5).clamp(0.1, 1.0);
        let color = match (state >> 8) % 7 { 0 => AMBER, 1 => RED, _ => GREEN };
        let bh = (level * (h - 2.0 * s)).max(2.0 * s);
        rrect(pix, x + i as f32 * (bw + gap), y + h - bh, bw, bh, (bw / 2.0).min(2.2 * s), color);
    }
}

const EVENTS: [(&str, &str); 15] = [
    ("54s", "test_failed"), ("58s", "test_failed"), ("1m", "test_started"), ("1m", "edit_finished"),
    ("1m", "command_started"), ("1m", "waiting"), ("1m", "test_failed"), ("1m", "test_started"),
    ("1m", "edit_finished"), ("1m", "command_started"), ("1m", "waiting"), ("1m", "test_failed"),
    ("1m", "test_started"), ("1m", "edit_finished"), ("2m", "command_started"),
];

fn header(pix: &mut Pixmap, g: &mut Glyphs, c: bool, s: f32) {
    rect(pix, 0.0, 0.0, 1480.0 * s, 56.0 * s, (0x11, 0x12, 0x14, 255));
    text(pix, g, c, 28.0 * s, 36.0 * s, 19.0 * s, "Activity monitor", (255, 255, 255));
    let mut x = 262.0 * s;
    for (label, color) in [("1 working", GREEN), ("1 waiting", AMBER), ("0 idle", (0x7d, 0x85, 0x94, 255)), ("0 stuck", RED), ("1 failed", RED), ("0 down", (0x7d, 0x85, 0x94, 255))] {
        let w = 90.0 * s;
        rrect(pix, x, 17.0 * s, w, 26.0 * s, 13.0 * s, (color.0, color.1, color.2, 44));
        text(pix, g, c, x + 24.0 * s, 35.0 * s, 14.0 * s, label, TEXT);
        x += w + 8.0 * s;
    }
}

fn rows(pix: &mut Pixmap, g: &mut Glyphs, c: bool, s: f32) {
    rect(pix, 0.0, 98.0 * s, 770.0 * s, 230.0 * s, (0x11, 0x12, 0x14, 255));
    for (i, (name, state, age)) in [("builder", "test_failed", "54s"), ("reviewer", "waiting", "53s"), ("worker", "command_started", "52s")].iter().enumerate() {
        let y = (108.0 + i as f32 * 54.0) * s;
        if i == 0 {
            rrect(pix, 12.0 * s, y, 754.0 * s, 46.0 * s, 10.0 * s, (0x1c, 0x23, 0x30, 255));
            rrect(pix, 12.0 * s, y + 7.0 * s, 3.0 * s, 32.0 * s, 1.5 * s, (0x5a, 0xa2, 0xff, 255));
        }
        text(pix, g, c, 64.0 * s, y + 29.0 * s, 17.0 * s, name, (255, 255, 255));
        text(pix, g, c, 196.0 * s, y + 29.0 * s, 15.0 * s, state, MUTED);
        text(pix, g, c, 470.0 * s, y + 29.0 * s, 15.0 * s, age, TEXT);
        lane(pix, s, 580.0 * s, y + 10.0 * s, 166.0 * s, 28.0 * s, 38, 11 + i as u32);
    }
}

fn detail(pix: &mut Pixmap, g: &mut Glyphs, c: bool, s: f32) {
    rect(pix, 780.0 * s, 56.0 * s, 700.0 * s, 700.0 * s, (0x11, 0x12, 0x14, 255));
    text(pix, g, c, 804.0 * s, 76.0 * s, 21.0 * s, "builder", (255, 255, 255));
    lane(pix, s, 804.0 * s, 138.0 * s, 640.0 * s, 60.0 * s, 96, 5);
    for (i, (age, name)) in EVENTS.iter().enumerate() {
        let y = (262.0 + i as f32 * 32.0) * s;
        if i % 2 == 0 {
            rrect(pix, 794.0 * s, y - 4.0 * s, 664.0 * s, 30.0 * s, 8.0 * s, (255, 255, 255, 6));
        }
        text(pix, g, c, 866.0 * s - 40.0 * s, y + 17.0 * s, 15.0 * s, age, MUTED);
        text(pix, g, c, 982.0 * s, y + 17.0 * s, 15.0 * s, name, TEXT);
        text(pix, g, c, 1180.0 * s, y + 17.0 * s, 14.0 * s, "hooks", MUTED);
    }
}

fn footer(pix: &mut Pixmap, g: &mut Glyphs, c: bool, s: f32) {
    rect(pix, 0.0, 766.0 * s, 1480.0 * s, 52.0 * s, (0x11, 0x12, 0x14, 255));
    let mut x = 24.0 * s;
    for k in ["move", "timeline", "details", "host", "filter", "sort", "IDs", "keys", "quit"] {
        rrect(pix, x, 784.0 * s, 28.0 * s, 22.0 * s, 6.0 * s, (0x1d, 0x20, 0x27, 255));
        text(pix, g, c, x + 36.0 * s, 800.0 * s, 14.0 * s, k, MUTED);
        x += 100.0 * s;
    }
}

fn full(pix: &mut Pixmap, g: &mut Glyphs, c: bool, s: f32) {
    pix.fill(Color::from_rgba8(0x11, 0x12, 0x14, 255));
    header(pix, g, c, s);
    rows(pix, g, c, s);
    detail(pix, g, c, s);
    footer(pix, g, c, s);
}

fn poll_update(pix: &mut Pixmap, g: &mut Glyphs, c: bool, s: f32) {
    rows(pix, g, c, s);
    let y = 262.0 * s;
    rect(pix, 794.0 * s, y - 4.0 * s, 664.0 * s, 30.0 * s, (0x11, 0x12, 0x14, 255));
    text(pix, g, c, 866.0 * s - 40.0 * s, y + 17.0 * s, 15.0 * s, "55s", MUTED);
    text(pix, g, c, 982.0 * s, y + 17.0 * s, 15.0 * s, "test_failed", TEXT);
}

fn stats(label: &str, mut samples: Vec<Duration>) {
    samples.sort();
    let n = samples.len();
    let ms = |d: Duration| d.as_secs_f64() * 1000.0;
    println!("{label:<46} median {:>7.3} ms   p95 {:>7.3} ms   max {:>7.3} ms   (n={n})", ms(samples[n / 2]), ms(samples[n * 95 / 100]), ms(samples[n - 1]));
}

fn time<F: FnMut()>(n: usize, mut f: F) -> Vec<Duration> {
    (0..n).map(|_| { let t = Instant::now(); f(); t.elapsed() }).collect()
}

fn main() {
    let font = std::env::args().nth(1).unwrap_or("/System/Library/Fonts/Supplemental/Arial.ttf".into());
    println!("scene: activity monitor mockup, 1480x818 logical; font {font}");
    for scale in [1.0f32, 2.0] {
        let (w, h) = ((1480.0 * scale) as u32, (818.0 * scale) as u32);
        println!("\n== scale {scale}x: {w}x{h} = {:.2} Mpx ==", (w * h) as f64 / 1e6);
        let mut pix = Pixmap::new(w, h).unwrap();
        let mut cold = Glyphs::new(&font);
        stats("full frame, glyph cache OFF (cold every time)", time(40, || full(&mut pix, &mut cold, false, scale)));
        let mut warm = Glyphs::new(&font);
        full(&mut pix, &mut warm, true, scale);
        stats("full frame, glyph cache warm", time(200, || full(&mut pix, &mut warm, true, scale)));
        stats("typical poll update (rows + 1 event), warm", time(400, || poll_update(&mut pix, &mut warm, true, scale)));
        stats("shapes only: both lanes areas (no text)", time(200, || {
            lane(&mut pix, scale, 804.0 * scale, 138.0 * scale, 640.0 * scale, 60.0 * scale, 96, 5);
            for i in 0..3 { lane(&mut pix, scale, 580.0 * scale, (118.0 + i as f32 * 54.0) * scale, 166.0 * scale, 28.0 * scale, 38, i); }
        }));
        let bytes = pix.data().to_vec();
        let mut copy = vec![0u8; bytes.len()];
        stats("handoff: memcpy of the frame", time(200, || copy.copy_from_slice(&bytes)));
        stats("handoff: SHA-256 of the frame", time(100, || { let mut h = Sha256::new(); h.update(&bytes); let _ = h.finalize(); }));
        let engine = base64::engine::general_purpose::STANDARD;
        stats("handoff: base64 of the frame (Kitty raw RGBA)", time(40, || { let _ = engine.encode(&bytes); }));
        let dirty = (770.0 * scale * 230.0 * scale * 4.0) as usize;
        stats("handoff: base64 of a 'rows' dirty region", time(100, || { let _ = engine.encode(&bytes[..dirty.min(bytes.len())]); }));
        println!("glyph cache warm: {} entries, hits {} misses {}", warm.cache.len(), warm.hits, warm.misses);
    }
}
