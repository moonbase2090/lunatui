// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use lunatui_badge::{Badge, Kbd, Look, Marker};
use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style};
use lunatui_image::{ImageRequest, ImageSink, ImageSource, PixelSize, PixelTarget};
use lunatui_runtime::RenderMode;
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_ui_tokens::Tone;
use lunatui_widgets::graphite::Face;
use lunatui_widgets::{RenderCx, Widget};

const COLUMNS: u16 = 100;
const ROWS: u16 = 5;
const CELL: (u32, u32) = (16, 32);
const FRAMES: usize = 8;

type Failure = Box<dyn Error>;

const COUNTS: [[u8; 6]; FRAMES] = [
    [3, 0, 1, 0, 0, 0],
    [2, 1, 1, 0, 0, 0],
    [2, 1, 0, 0, 1, 0],
    [1, 1, 0, 1, 1, 0],
    [1, 2, 0, 0, 1, 0],
    [1, 1, 0, 0, 1, 1],
    [2, 0, 0, 0, 1, 1],
    [3, 1, 0, 0, 0, 0],
];

const STATES: [(&str, Tone, Marker); 6] = [
    ("working", Tone::Success, Marker::Dot),
    ("waiting", Tone::Warning, Marker::Diamond),
    ("idle", Tone::Neutral, Marker::Ring),
    ("stuck", Tone::Warning, Marker::Bang),
    ("failed", Tone::Danger, Marker::Cross),
    ("down", Tone::Neutral, Marker::Square),
];

const HINTS: [(&[&str], &str); 8] = [
    (&["↑", "↓"], "move"),
    (&["⏎"], "timeline"),
    (&["d"], "details"),
    (&["Tab"], "host"),
    (&["f"], "filter"),
    (&["s"], "sort"),
    (&["?"], "keys"),
    (&["q"], "quit"),
];

#[derive(Default)]
struct Host {
    background: Option<[u8; 3]>,
    images: Vec<(Rect, ImageSource)>,
}

impl ImageSink for Host {
    fn supports_pixels(&self) -> bool {
        true
    }

    fn pixel_target(&self, area: Rect) -> Option<PixelTarget> {
        let size = PixelSize::new(
            u32::from(area.width) * CELL.0,
            u32::from(area.height) * CELL.1,
        )?;
        Some(PixelTarget::new(size, false, self.background))
    }

    fn submit(&mut self, _: ImageRequest<'_>) -> bool {
        false
    }

    fn submit_raster(&mut self, source: &ImageSource, area: Rect, _: i32) -> bool {
        self.images.push((area, source.clone()));
        true
    }
}

fn label(cx: &mut RenderCx<'_>, area: Rect, text: &str, style: Style, size: f32) {
    let face = if style.attrs.bold() {
        Face::SemiBold
    } else {
        Face::Regular
    };
    let drawn = cx.replace_with_pixels(area, |px| {
        let height = f32::from(px.cell_size().height_px());
        let size = height * size;
        let baseline = px.line_metrics(face, size).map_or(height * 0.7, |line| {
            (height + line.ascent - line.descent) / 2.0
        });
        px.text(face, size, (0.0, baseline), text, style.fg);
    });
    if !drawn {
        cx.put_str(area.x, area.y, text, style);
    }
}

fn draw(cx: &mut RenderCx<'_>, counts: [u8; 6]) {
    let area = Rect::new(0, 0, COLUMNS, ROWS);
    let background = Style::new().bg(cx.token(Token::Surface0));
    cx.fill(area, background, ' ');
    let title = background
        .fg(cx.token(Token::TextPrimary))
        .with_attrs(Attrs::new().set_bold(true));
    label(cx, Rect::new(2, 1, 17, 1), "Activity monitor", title, 0.62);
    let mut x = 21;
    for ((name, tone, marker), count) in STATES.iter().zip(counts) {
        let text = format!("{count} {name}");
        let badge = Badge::new(&text)
            .tone(*tone)
            .look(Look::Subtle)
            .marker(*marker);
        let width = badge.measure(lunatui_core::Size::new(COLUMNS - x, 1)).width;
        badge.render(Rect::new(x, 1, width, 1), cx);
        x += width + 1;
    }
    let muted = background.fg(cx.token(Token::TextMuted));
    let mut x = 2;
    for (keys, action) in HINTS {
        let kbd = Kbd::new(keys);
        let width = kbd.measure(lunatui_core::Size::new(COLUMNS - x, 1)).width;
        kbd.render(Rect::new(x, 3, width, 1), cx);
        x += width + 1;
        let cells = u16::try_from(action.len()).unwrap_or(0);
        label(cx, Rect::new(x, 3, cells, 1), action, muted, 0.56);
        x += cells + 2;
    }
}

fn rgb(color: Color) -> Result<[u8; 3], Failure> {
    match color {
        Color::Rgb(red, green, blue) => Ok([red, green, blue]),
        other => Err(format!("{other:?} is not an RGB color").into()),
    }
}

fn covers(area: Rect, column: u16, row: u16) -> bool {
    (area.x..area.x.saturating_add(area.width)).contains(&column)
        && (area.y..area.y.saturating_add(area.height)).contains(&row)
}

fn composite(grid: &Grid, clusters: &ClusterTable, host: &Host) -> Result<Vec<u8>, Failure> {
    let (width, height) = (u32::from(COLUMNS) * CELL.0, u32::from(ROWS) * CELL.1);
    let mut frame = vec![0_u8; usize::try_from(width * height * 4)?];
    for row in 0..ROWS {
        for column in 0..COLUMNS {
            let cell = grid.get(column, row).ok_or("cell outside the grid")?;
            let covered = host
                .images
                .iter()
                .any(|(area, _)| covers(*area, column, row));
            if !covered && cell.glyph().decode() != lunatui_core::DecodedGlyph::Inline(' ') {
                return Err(format!(
                    "cell text left outside an image at {column},{row}: {:?}",
                    grid.row_display(row, clusters)
                )
                .into());
            }
            let [red, green, blue] = rgb(cell.style().bg)?;
            for y in 0..CELL.1 {
                let start = usize::try_from(
                    ((u32::from(row) * CELL.1 + y) * width + u32::from(column) * CELL.0) * 4,
                )?;
                for pixel in frame[start..start + usize::try_from(CELL.0 * 4)?]
                    .as_chunks_mut::<4>()
                    .0
                    .iter_mut()
                {
                    *pixel = [red, green, blue, 255];
                }
            }
        }
    }
    for (area, image) in &host.images {
        let stride = usize::try_from(image.size().width() * 4)?;
        for (y, line) in image.rgba8_pixels().chunks_exact(stride).enumerate() {
            let top = usize::try_from(u32::from(area.y) * CELL.1)? + y;
            let start =
                (top * usize::try_from(width)? + usize::try_from(u32::from(area.x) * CELL.0)?) * 4;
            frame[start..start + stride].copy_from_slice(line);
        }
    }
    Ok(frame)
}

fn graphite(out: &Path, theme: &Theme) -> Result<(), Failure> {
    for (index, counts) in COUNTS.iter().enumerate() {
        let mut grid = Grid::new(COLUMNS, ROWS);
        let mut clusters = ClusterTable::new();
        let mut host = Host {
            background: Some(rgb(theme.resolve(Token::Surface0))?),
            images: Vec::new(),
        };
        {
            let mut cx = RenderCx::new(&mut grid, &mut clusters, theme)
                .image_sink(&mut host)
                .render_mode(RenderMode::Graphite);
            draw(&mut cx, *counts);
        }
        let frame = composite(&grid, &clusters, &host)?;
        let path = out.join(format!("graphite-{index:02}.png"));
        image::save_buffer(
            &path,
            &frame,
            u32::from(COLUMNS) * CELL.0,
            u32::from(ROWS) * CELL.1,
            image::ExtendedColorType::Rgba8,
        )?;
        println!("wrote {}", path.display());
    }
    Ok(())
}

fn cells(out: &Path, theme: &Theme) -> Result<(), Failure> {
    let mut encoder = Encoder::new();
    let mut cast = format!(
        "{{\"version\": 2, \"width\": {COLUMNS}, \"height\": {ROWS}, \"timestamp\": 0, \"env\": {{\"TERM\": \"xterm-256color\"}}}}\n"
    );
    for (index, counts) in COUNTS.iter().enumerate() {
        let mut grid = Grid::new(COLUMNS, ROWS);
        let mut clusters = ClusterTable::new();
        draw(&mut RenderCx::new(&mut grid, &mut clusters, theme), *counts);
        let bytes = encoder.encode_diff(
            &Grid::new(COLUMNS, ROWS),
            &grid,
            &clusters,
            ColorMode::TrueColor,
        );
        let text = format!(
            "\u{1b}[0m\u{1b}[2J\u{1b}[H{}",
            String::from_utf8_lossy(bytes)
        );
        let event = serde_json::json!([u32::try_from(index)?, "o", text]);
        cast.push_str(&event.to_string());
        cast.push('\n');
    }
    let path = out.join("cells.cast");
    fs::write(&path, cast)?;
    println!("wrote {}", path.display());
    Ok(())
}

fn main() -> Result<(), Failure> {
    let out = PathBuf::from(
        env::args()
            .nth(1)
            .ok_or("usage: graphite_chips OUTPUT_DIR")?,
    );
    fs::create_dir_all(&out)?;
    let theme = Theme::builtin(Variant::Dark);
    cells(&out, &theme)?;
    graphite(&out, &theme)
}
