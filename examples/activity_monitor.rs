// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use lunatui_core::{ClusterTable, Color, Grid, Rect};
use lunatui_image::{ImageRequest, ImageSink, ImageSource, PixelSize, PixelTarget};
use lunatui_runtime::RenderMode;
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::RenderCx;

#[path = "activity_monitor_scene.rs"]
mod scene;

use scene::{COLUMNS, ROWS, draw};

const CELL: (u32, u32) = (16, 32);
const FRAMES: usize = 10;

type Failure = Box<dyn Error>;

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
    let stride = usize::try_from(width)? * 4;
    let mut frame = vec![0_u8; stride * usize::try_from(height)?];
    let cell_bytes = usize::try_from(CELL.0)? * 4;
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
            for y in 0..usize::try_from(CELL.1)? {
                let top = usize::from(row) * usize::try_from(CELL.1)? + y;
                let start = top * stride + usize::from(column) * cell_bytes;
                for pixel in frame[start..start + cell_bytes].as_chunks_mut::<4>().0 {
                    *pixel = [red, green, blue, 255];
                }
            }
        }
    }
    for (area, image) in &host.images {
        let line_bytes = usize::try_from(image.size().width())? * 4;
        for (y, line) in image.rgba8_pixels().chunks_exact(line_bytes).enumerate() {
            let top = usize::from(area.y) * usize::try_from(CELL.1)? + y;
            let start = top * stride + usize::from(area.x) * cell_bytes;
            frame[start..start + line_bytes].copy_from_slice(line);
        }
    }
    Ok(frame)
}

fn graphite(out: &Path, theme: &Theme) -> Result<(), Failure> {
    for index in 0..FRAMES {
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
            draw(&mut cx, index);
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
    for index in 0..FRAMES {
        let mut grid = Grid::new(COLUMNS, ROWS);
        let mut clusters = ClusterTable::new();
        draw(&mut RenderCx::new(&mut grid, &mut clusters, theme), index);
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
            .ok_or("usage: activity_monitor OUTPUT_DIR")?,
    );
    fs::create_dir_all(&out)?;
    let theme = Theme::builtin(Variant::Dark);
    cells(&out, &theme)?;
    graphite(&out, &theme)
}
