// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_image::{ImageRequest, ImageSink, ImageSource, PixelSize, PixelTarget};
use lunatui_runtime::RenderMode;
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::graphite::{Face, Rect as PixelRect, UiFont};
use lunatui_widgets::{PixelCx, RenderCx};

const CELL: (u32, u32) = (16, 32);
const COLUMNS: u16 = 40;
const ROWS: u16 = 48;
const PAD: f32 = 32.0;
const GAP: u32 = 24;

type Rgb = (u8, u8, u8);
type Failure = Box<dyn Error>;

const BACKGROUND: Rgb = (0x14, 0x16, 0x1a);
const SURFACE: Rgb = (0x1b, 0x1e, 0x24);
const MENU_SURFACE: Rgb = (0x20, 0x24, 0x2b);
const HAIRLINE: Rgb = (0x2c, 0x31, 0x3b);
const TEXT: Rgb = (0xe6, 0xe9, 0xee);
const STRONG: Rgb = (0xff, 0xff, 0xff);
const MUTED: Rgb = (0xa3, 0xab, 0xb8);
const SELECT: Rgb = (0x25, 0x2a, 0x33);
const FOCUS: Rgb = (0x5a, 0xa2, 0xff);
const ON_FOCUS: Rgb = (0x0b, 0x12, 0x20);
const GREEN: Rgb = (0x4c, 0xc9, 0x8a);
const AMBER: Rgb = (0xf2, 0xb8, 0x4b);
const RED: Rgb = (0xff, 0x7a, 0x6b);

const PANELS: [(UiFont, &str, &str); 3] = [
    (UiFont::Inter, "Inter", "inter"),
    (UiFont::IbmPlexSans, "IBM Plex Sans", "ibm-plex-sans"),
    (UiFont::NotoSans, "Noto Sans", "noto-sans"),
];

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

fn rect(left: f32, top: f32, width: f32, height: f32) -> PixelRect {
    let pixels = |value: f32| {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "layout values are small positive numbers"
        )]
        let rounded = value.round().max(0.0) as usize;
        rounded
    };
    PixelRect {
        x: pixels(left),
        y: pixels(top),
        w: pixels(width),
        h: pixels(height),
    }
}

struct Pen<'p, 'c> {
    px: &'p mut PixelCx<'c>,
    width: f32,
    y: f32,
}

impl Pen<'_, '_> {
    fn line(&mut self, face: Face, size: f32, text: &str, ink: Rgb) {
        let ascent = self
            .px
            .line_metrics(face, size)
            .map_or(size, |line| line.ascent);
        self.px
            .text(face, size, (PAD, self.y + ascent), text, color(ink));
        self.y += size * 1.45;
    }

    fn rule(&mut self) {
        self.y += 8.0;
        self.px
            .fill_rect(rect(0.0, self.y, self.width, 2.0), 0.0, color(HAIRLINE));
        self.y += 24.0;
    }

    fn heading(&mut self, text: &str) {
        self.line(Face::SemiBold, 20.0, text, MUTED);
        self.y += 4.0;
    }

    fn menu(&mut self) {
        let items = [
            ("Open project…", "Ctrl O", false),
            ("Recent projects", "›", true),
            ("Save all", "Ctrl S", false),
            ("Close window", "Ctrl W", false),
        ];
        let row = 44.0;
        let height = row * 4.0 + 16.0;
        let inner = self.width - 2.0 * PAD;
        self.px.fill_rect(
            rect(PAD, self.y, inner, height),
            12.0,
            color(MENU_SURFACE),
        );
        self.px.stroke_rect(
            rect(PAD, self.y, inner, height),
            12.0,
            2.0,
            color(HAIRLINE),
        );
        let mut top = self.y + 8.0;
        for (label, key, selected) in items {
            let (ink, hint) = if selected {
                self.px.fill_rect(
                    rect(PAD + 8.0, top, inner - 16.0, row),
                    8.0,
                    color(FOCUS),
                );
                (ON_FOCUS, ON_FOCUS)
            } else {
                (TEXT, MUTED)
            };
            let baseline = top + 30.0;
            self.px.text(
                Face::Regular,
                26.0,
                (PAD + 24.0, baseline),
                label,
                color(ink),
            );
            let key_width = self.px.measure(Face::Regular, 22.0, key);
            self.px.text(
                Face::Regular,
                22.0,
                (PAD + inner - 24.0 - key_width, baseline),
                key,
                color(hint),
            );
            top += row;
        }
        self.y += height + 24.0;
    }

    fn services(&mut self) {
        let rows = [
            ("api-gateway", "Healthy", GREEN, false),
            ("worker-pool", "Degraded · 3 of 8 lagging", AMBER, true),
            ("billing-reconciliation-service-eu-west", "Failed", RED, false),
        ];
        let inner = self.width - 2.0 * PAD;
        for (name, state, dot, selected) in rows {
            if selected {
                self.px
                    .fill_rect(rect(PAD, self.y, inner, 64.0), 8.0, color(SELECT));
            }
            self.px.fill_rect(
                rect(PAD + 16.0, self.y + 26.0, 12.0, 12.0),
                6.0,
                color(dot),
            );
            let room = inner - 64.0;
            let name = self.px.fit(Face::SemiBold, 24.0, name, room);
            self.px.text(
                Face::SemiBold,
                24.0,
                (PAD + 44.0, self.y + 28.0),
                &name,
                color(STRONG),
            );
            self.px.text(
                Face::Regular,
                20.0,
                (PAD + 44.0, self.y + 54.0),
                state,
                color(MUTED),
            );
            self.y += 72.0;
        }
        self.y += 8.0;
    }

    fn code(&mut self) {
        let lines = [
            "fn main() -> Result<(), Error> {",
            "    let total = 1_024 * 0x2f; // 0O 1lI",
            "    println!(\"{total:>8} => ok\");",
            "}",
        ];
        let inner = self.width - 2.0 * PAD;
        let height = 36.0 * 4.0 + 24.0;
        self.px
            .fill_rect(rect(PAD, self.y, inner, height), 10.0, color(SURFACE));
        let mut baseline = self.y + 38.0;
        for line in lines {
            self.px.text(
                Face::Mono,
                21.0,
                (PAD + 16.0, baseline),
                line,
                color(TEXT),
            );
            baseline += 36.0;
        }
        self.y += height + 24.0;
    }
}

fn draw(px: &mut PixelCx<'_>, title: &str) {
    let width = f32::from(u16::try_from(px.width()).unwrap_or(u16::MAX));
    let mut pen = Pen { px, width, y: PAD };
    pen.line(Face::SemiBold, 40.0, title, STRONG);
    pen.line(
        Face::Regular,
        22.0,
        "Bundled font, drawn by PixelCx at 2x",
        MUTED,
    );
    pen.rule();
    pen.heading("MENU");
    pen.menu();
    pen.heading("SERVICES");
    pen.services();
    pen.rule();
    pen.heading("PARAGRAPH");
    for line in [
        "The rollout paused at 40% because two",
        "canaries reported elevated p99 latency.",
        "Resume it once the error budget recovers.",
    ] {
        pen.line(Face::Regular, 26.0, line, TEXT);
    }
    pen.y += 8.0;
    pen.heading("NUMBERS AND PUNCTUATION");
    for size in [20.0, 26.0, 32.0, 38.0] {
        pen.line(Face::Regular, size, "0123456789 $1,284.50 (+3.2%)", TEXT);
    }
    pen.line(
        Face::Regular,
        26.0,
        "“Quotes” — ‘dashes’ – & @ # / … ? !",
        TEXT,
    );
    pen.y += 8.0;
    pen.heading("MONOSPACED");
    pen.code();
    pen.heading("FALLBACK");
    pen.line(
        Face::Regular,
        26.0,
        "Greek αβγδ  Cyrillic жзий  ← → ↑ ↓",
        TEXT,
    );
    pen.line(Face::Regular, 26.0, "Blocks ▁▂▃▄▅▆▇█ ░▒▓ ┌─┬─┐ ✓ ✗", TEXT);
    pen.line(Face::Regular, 30.0, "Emoji 🚀 ✅ ⚠️ 🔥 📦 🙂", TEXT);
}

fn render(ui: UiFont, title: &str) -> Result<ImageSource, Failure> {
    let area = Rect::new(0, 0, COLUMNS, ROWS);
    let mut grid = Grid::new(COLUMNS, ROWS);
    let mut clusters = ClusterTable::new();
    grid.fill(
        area,
        Style::new().bg(color(BACKGROUND)),
        ' ',
        &mut clusters,
    );
    let theme = Theme::builtin(Variant::Dark);
    let mut capture = Capture { image: None };
    let painted = RenderCx::new(&mut grid, &mut clusters, &theme)
        .image_sink(&mut capture)
        .render_mode(RenderMode::Graphite)
        .ui_font(ui)
        .paint_pixels(area, 0, |px| draw(px, title));
    match capture.image {
        Some(image) if painted => Ok(image),
        _ => Err("the panel was not painted".into()),
    }
}

fn side_by_side(panels: &[ImageSource]) -> Result<(u32, u32, Vec<u8>), Failure> {
    let panel_width = COLUMNS_PX;
    let height = ROWS_PX;
    let count = u32::try_from(panels.len())?;
    let width = panel_width * count + GAP * (count + 1);
    let full = height + 2 * GAP;
    let (red, green, blue) = BACKGROUND;
    let mut sheet = [red, green, blue, 255].repeat(usize::try_from(width * full)?);
    let stride = usize::try_from(width)? * 4;
    let row_bytes = usize::try_from(panel_width)? * 4;
    for (index, panel) in (0_u32..).zip(panels) {
        let left = usize::try_from(GAP + index * (panel_width + GAP))? * 4;
        for (row, line) in panel.rgba8_pixels().chunks_exact(row_bytes).enumerate() {
            let start = (usize::try_from(GAP)? + row) * stride + left;
            sheet[start..start + row_bytes].copy_from_slice(line);
        }
    }
    Ok((width, full, sheet))
}

const COLUMNS_PX: u32 = COLUMNS as u32 * CELL.0;
const ROWS_PX: u32 = ROWS as u32 * CELL.1;

fn save(path: PathBuf, width: u32, height: u32, rgba: &[u8]) -> Result<(), Failure> {
    image::save_buffer(&path, rgba, width, height, image::ExtendedColorType::Rgba8)?;
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
            out.join(format!("font-{slug}.png")),
            COLUMNS_PX,
            ROWS_PX,
            panel.rgba8_pixels(),
        )?;
        panels.push(panel);
    }
    let (width, height, sheet) = side_by_side(&panels)?;
    save(out.join("side-by-side.png"), width, height, &sheet)
}
