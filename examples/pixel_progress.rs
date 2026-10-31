// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::io;

use lunatui::{App, ControlFlow, RunOptions, is_quit, run};
use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_runtime::RenderFrame;
use lunatui_term::{Event, KeyCode};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Paragraph, RenderCx, Widget, WidgetState};

mod progress_scene;

const WIDTH: u16 = 248;
const HEIGHT: u16 = 44;
const CASES: [(&str, u32, WidgetState); 8] = [
    ("Waiting", 0, WidgetState::Normal),
    ("Checkout", 31, WidgetState::Normal),
    ("Compile", 100, WidgetState::Normal),
    ("Tests", 140, WidgetState::Normal),
    ("Upload", 215, WidgetState::Normal),
    ("Complete", 240, WidgetState::Normal),
    ("Focused", 215, WidgetState::Focused),
    ("Disabled", 215, WidgetState::Disabled),
];

fn main() -> io::Result<()> {
    run(
        Screen {
            page: 0,
            sample: 0,
            pixels: true,
        },
        RunOptions::new(WIDTH, HEIGHT),
    )
}

struct Screen {
    page: usize,
    sample: u32,
    pixels: bool,
}

impl Screen {
    fn draw(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let style = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::Surface0));
        let accent = style.fg(cx.token(Token::Accent));
        let blank = vec![" ".repeat(usize::from(area.width)); usize::from(area.height)].join("\n");
        Paragraph::new(blank).style(style).render(area, cx);
        Paragraph::new(format!("PROGRESS BAR  /  {}", CASES[self.page].0))
            .style(accent)
            .render(Rect::new(3, 1, 242, 1), cx);
        Paragraph::new("← → select stage    u advance sample    b toggle right pane    q quit")
            .style(style)
            .render(Rect::new(3, 42, 242, 1), cx);
        let (_, tick, state) = CASES[self.page];
        for index in 0..2_u16 {
            let left = 2 + index * 124;
            let pixels = index == 1 && self.pixels;
            Paragraph::new(if pixels { "Pixels" } else { "Cells" })
                .style(accent)
                .render(Rect::new(left + 1, 3, 118, 1), cx);
            progress_scene::draw(
                Rect::new(left, 5, 120, 36),
                tick.wrapping_add(self.sample),
                pixels,
                state,
                cx,
            );
        }
    }
}

impl App for Screen {
    fn update(&mut self, event: &Event) -> ControlFlow {
        if is_quit(event) {
            return ControlFlow::Break;
        }
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Right => {
                    self.page = (self.page + 1) % CASES.len();
                    self.sample = 0;
                }
                KeyCode::Left => {
                    self.page = (self.page + CASES.len() - 1) % CASES.len();
                    self.sample = 0;
                }
                KeyCode::Char('u') => self.sample = self.sample.wrapping_add(1),
                KeyCode::Char('b') => self.pixels = !self.pixels,
                _ => {}
            }
        }
        ControlFlow::Continue
    }
    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        self.draw(
            area,
            &mut RenderCx::new(grid, clusters, &Theme::builtin(Variant::Dark)),
        );
    }
    fn render_frame(&mut self, frame: &mut RenderFrame<'_>, area: Rect) {
        let theme = Theme::builtin(Variant::Dark);
        let (grid, clusters, images) = frame.parts();
        self.draw(
            area,
            &mut RenderCx::new(grid, clusters, &theme).image_sink(images),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lunatui_image::{ImageRequest, ImageSink, ImageSource, PixelSize, PixelTarget};

    struct Capture {
        images: Vec<(ImageSource, Rect)>,
    }
    impl ImageSink for Capture {
        fn supports_pixels(&self) -> bool {
            true
        }
        fn pixel_target(&self, area: Rect) -> Option<PixelTarget> {
            PixelSize::new(u32::from(area.width) * 11, u32::from(area.height) * 25)
                .map(|size| PixelTarget::new(size, false, Some([11, 22, 33])))
        }
        fn submit(&mut self, _: ImageRequest<'_>) -> bool {
            false
        }
        fn submit_raster(&mut self, source: &ImageSource, area: Rect, depth: i32) -> bool {
            assert_eq!(depth, 0);
            self.images.push((source.clone(), area));
            true
        }
    }
    fn frame(screen: &Screen, grid: &mut Grid, clusters: &mut ClusterTable) -> Capture {
        let mut capture = Capture { images: Vec::new() };
        screen.draw(
            Rect::new(0, 0, WIDTH, HEIGHT),
            &mut RenderCx::new(grid, clusters, &Theme::builtin(Variant::Dark))
                .image_sink(&mut capture),
        );
        capture
    }
    #[test]
    fn every_case_keeps_the_full_pipeline_and_only_right_tracks_use_images() {
        let mut grid = Grid::new(WIDTH, HEIGHT);
        let mut clusters = ClusterTable::new();
        for page in 0..CASES.len() {
            let capture = frame(
                &Screen {
                    page,
                    sample: 0,
                    pixels: true,
                },
                &mut grid,
                &mut clusters,
            );
            assert_eq!(capture.images.len(), 6);
            assert_eq!(
                capture
                    .images
                    .iter()
                    .map(|(_, area)| *area)
                    .collect::<Vec<_>>(),
                vec![
                    Rect::new(131, 10, 63, 1),
                    Rect::new(131, 14, 63, 1),
                    Rect::new(131, 18, 63, 1),
                    Rect::new(131, 22, 63, 1),
                    Rect::new(131, 26, 68, 1),
                    Rect::new(137, 32, 102, 1)
                ]
            );
            for (row, text) in [
                (5, "Release pipeline  ·  lunatui v0.1.15"),
                (9, "Checkout"),
                (13, "Compile"),
                (17, "Test suites"),
                (21, "Package"),
                (25, "Upload"),
                (11, "14 files"),
                (19, "1,087 tests"),
                (27, "uploading to the release"),
                (32, "Overall"),
                (40, "r rerun stage"),
            ] {
                assert_eq!(
                    grid.row_display(row, &clusters).matches(text).count(),
                    2,
                    "page {page}, row {row}"
                );
            }
            for y in 5..41 {
                for x in 0..120 {
                    let a = grid.get(2 + x, y);
                    let b = grid.get(126 + x, y);
                    assert_eq!(
                        a.map(lunatui_core::Cell::style),
                        b.map(lunatui_core::Cell::style),
                        "page {page}, {x},{y}"
                    );
                    assert_eq!(
                        a.map(lunatui_core::Cell::meta),
                        b.map(lunatui_core::Cell::meta)
                    );
                    let native = capture
                        .images
                        .iter()
                        .any(|(_, r)| y == r.y && 126 + x >= r.x && 126 + x < r.x + r.width);
                    if !native {
                        assert_eq!(a, b, "page {page}, {x},{y}");
                    }
                }
            }
        }
    }
    #[test]
    fn off_and_on_restore_the_same_sources_and_cells_on_a_reused_frame() {
        let mut grid = Grid::new(WIDTH, HEIGHT);
        let mut clusters = ClusterTable::new();
        let screen = Screen {
            page: 7,
            sample: 0,
            pixels: true,
        };
        let before = frame(&screen, &mut grid, &mut clusters);
        let off = frame(
            &Screen {
                pixels: false,
                ..screen
            },
            &mut grid,
            &mut clusters,
        );
        assert_eq!(off.images.len(), 0);
        for y in 5..41 {
            for x in 0..120 {
                assert_eq!(grid.get(2 + x, y), grid.get(126 + x, y));
            }
        }
        assert!(grid.row_display(26, &clusters).contains('█'));
        let after = frame(&screen, &mut grid, &mut clusters);
        assert_eq!(after.images.len(), 6);
        for ((a, area), (b, other)) in before.images.iter().zip(&after.images) {
            assert_eq!(area, other);
            assert_eq!(a.rgba8_pixels(), b.rgba8_pixels());
        }
    }
}
