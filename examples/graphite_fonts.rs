// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_image::{ImageRequest, ImageSink, ImageSource, PixelSize, PixelTarget};
use lunatui_runtime::RenderMode;
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::graphite::{Face, UiFont};
use lunatui_widgets::{Block, PixelCx, RenderCx, Widget};

const CELL: (u32, u32) = (16, 32);
const COLUMNS: u16 = 40;
const ROWS: u16 = 33;
const PAD: f32 = 32.0;
const GAP: u32 = 24;
const COLUMNS_PX: u32 = COLUMNS as u32 * CELL.0;
const ROWS_PX: u32 = ROWS as u32 * CELL.1;

type Rgb = (u8, u8, u8);
type Failure = Box<dyn Error>;

const BACKGROUND: Rgb = (0x14, 0x16, 0x1a);
const TEXT: Rgb = (0xe6, 0xe9, 0xee);
const STRONG: Rgb = (0xff, 0xff, 0xff);
const MUTED: Rgb = (0xa3, 0xab, 0xb8);

const PANELS: [(UiFont, &str, &str); 4] = [
    (UiFont::Inter, "Inter", "inter"),
    (UiFont::IbmPlexSans, "IBM Plex Sans", "ibm-plex-sans"),
    (UiFont::NotoSans, "Noto Sans", "noto-sans"),
    (UiFont::InterCompact, "Inter compact", "inter-compact"),
];

const PANGRAM: &str = "The quick brown fox jumps over the lazy dog.";
const LONG_NAME: &str = "billing-reconciliation-service-eu-west-2";

struct Capture {
    image: Option<ImageSource>,
}

impl ImageSink for Capture {
    fn supports_pixels(&self) -> bool {
        true
    }

    fn pixel_target(&self, area: Rect) -> Option<PixelTarget> {
        let size = PixelSize::new(
            u32::from(area.width) * CELL.0,
            u32::from(area.height) * CELL.1,
        )?;
        Some(PixelTarget::new(size, false, None))
    }

    fn submit(&mut self, _: ImageRequest<'_>) -> bool {
        false
    }

    fn submit_raster(&mut self, source: &ImageSource, _: Rect, _: i32) -> bool {
        self.image = Some(source.clone());
        true
    }
}

fn color((red, green, blue): Rgb) -> Color {
    Color::Rgb(red, green, blue)
}

struct Specimen<'p, 'c> {
    px: &'p mut PixelCx<'c>,
    y: f32,
}

impl Specimen<'_, '_> {
    fn line(&mut self, face: Face, size: f32, text: &str, ink: Rgb) {
        let ascent = self
            .px
            .line_metrics(face, size)
            .map_or(size, |line| line.ascent);
        self.px
            .text(face, size, (PAD, self.y + ascent), text, color(ink));
        self.y += size * 1.4;
    }

    fn label(&mut self, text: &str) {
        self.y += 14.0;
        self.line(Face::SemiBold, 18.0, text, MUTED);
        self.y += 2.0;
    }
}

fn draw(px: &mut PixelCx<'_>, title: &str) {
    let width = f32::from(u16::try_from(px.width()).unwrap_or(u16::MAX));
    let mut specimen = Specimen { px, y: PAD };
    specimen.line(Face::SemiBold, 44.0, title, STRONG);
    specimen.line(
        Face::Regular,
        20.0,
        "UI font, with JetBrains Mono for Face::Mono",
        MUTED,
    );
    specimen.label("REGULAR AND SEMIBOLD, 26 PX");
    specimen.line(Face::Regular, 26.0, PANGRAM, TEXT);
    specimen.line(Face::SemiBold, 26.0, PANGRAM, TEXT);
    specimen.label("SIZES 16 TO 40 PX");
    for size in [16.0, 20.0, 26.0, 32.0, 40.0] {
        specimen.line(Face::Regular, size, "Rollout 40% · p99 1,284 ms", TEXT);
    }
    specimen.label("DIGITS AND PUNCTUATION");
    specimen.line(Face::Regular, 26.0, "0123456789 $1,284.50 (+3.2%)", TEXT);
    specimen.line(
        Face::Regular,
        26.0,
        "“Quotes” — ‘dashes’ – & @ # / … ? !",
        TEXT,
    );
    specimen.label("FIT TO THE PANEL WIDTH");
    let room = width - 2.0 * PAD;
    let fitted = specimen
        .px
        .fit(Face::SemiBold, 40.0, LONG_NAME, room)
        .into_owned();
    specimen.line(Face::SemiBold, 40.0, &fitted, TEXT);
    specimen.label("FACE::MONO");
    specimen.line(Face::Mono, 22.0, "fn main() -> Result<(), Error> {", TEXT);
    specimen.line(
        Face::Mono,
        22.0,
        "    let total = 1_024 * 0x2f; // 0O 1lI",
        TEXT,
    );
    specimen.line(Face::Mono, 22.0, "}", TEXT);
    specimen.label("FALLBACK");
    specimen.line(
        Face::Regular,
        26.0,
        "Greek αβγδ  Cyrillic жзий  ← → ↑ ↓",
        TEXT,
    );
    specimen.line(Face::Regular, 26.0, "Blocks ▁▂▃▄▅▆▇█ ░▒▓ ┌─┬─┐ ✓ ✗", TEXT);
    specimen.line(Face::Regular, 30.0, "Emoji 🚀 ✅ ⚠️ 🔥 📦 🙂", TEXT);
}

fn render(ui: UiFont, title: &str) -> Result<ImageSource, Failure> {
    let area = Rect::new(0, 0, COLUMNS, ROWS);
    let mut grid = Grid::new(COLUMNS, ROWS);
    let mut clusters = ClusterTable::new();
    let theme = Theme::builtin(Variant::Dark);
    let mut capture = Capture { image: None };
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme)
        .image_sink(&mut capture)
        .render_mode(RenderMode::Graphite)
        .ui_font(ui);
    Block::new()
        .style(Style::new().bg(color(BACKGROUND)))
        .render(area, &mut cx);
    let painted = cx.paint_pixels(area, 0, |px| draw(px, title));
    match capture.image {
        Some(image) if painted => Ok(image),
        _ => Err("the specimen was not painted".into()),
    }
}

fn side_by_side(panels: &[ImageSource]) -> Result<(u32, u32, Vec<u8>), Failure> {
    let count = u32::try_from(panels.len())?;
    let width = COLUMNS_PX * count + GAP * (count + 1);
    let height = ROWS_PX + 2 * GAP;
    let (red, green, blue) = BACKGROUND;
    let mut sheet = [red, green, blue, 255].repeat(usize::try_from(width * height)?);
    let stride = usize::try_from(width)? * 4;
    let row_bytes = usize::try_from(COLUMNS_PX)? * 4;
    for (index, panel) in (0_u32..).zip(panels) {
        let left = usize::try_from(GAP + index * (COLUMNS_PX + GAP))? * 4;
        for (row, line) in panel.rgba8_pixels().chunks_exact(row_bytes).enumerate() {
            let start = (usize::try_from(GAP)? + row) * stride + left;
            sheet[start..start + row_bytes].copy_from_slice(line);
        }
    }
    Ok((width, height, sheet))
}

fn save(path: &Path, width: u32, height: u32, rgba: &[u8]) -> Result<(), Failure> {
    image::save_buffer(path, rgba, width, height, image::ExtendedColorType::Rgba8)?;
    println!("wrote {}", path.display());
    Ok(())
}

fn main() -> Result<(), Failure> {
    let out = PathBuf::from(
        env::args()
            .nth(1)
            .ok_or("usage: graphite_fonts OUTPUT_DIR")?,
    );
    fs::create_dir_all(&out)?;
    let mut panels = Vec::new();
    for (ui, title, slug) in PANELS {
        let panel = render(ui, title)?;
        save(
            &out.join(format!("font-{slug}.png")),
            COLUMNS_PX,
            ROWS_PX,
            panel.rgba8_pixels(),
        )?;
        panels.push(panel);
    }
    let (width, height, sheet) = side_by_side(&panels)?;
    save(&out.join("side-by-side.png"), width, height, &sheet)
}
