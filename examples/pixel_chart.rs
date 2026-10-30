// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::io;

use lunatui::{App, ControlFlow, RunOptions, is_quit, run};
use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_runtime::RenderFrame;
use lunatui_term::{Event, KeyCode};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Block, Paragraph, RenderCx, Widget};

mod chart_scene;
use chart_scene::ChartBoard;

const WIDTH: u16 = 100;
const HEIGHT: u16 = 32;
const CASES: [&str; 4] = ["Requests", "Saturation", "Latency scatter", "Hourly bars"];

fn main() -> io::Result<()> {
    run(Screen { page: 0, tick: 0 }, RunOptions::new(WIDTH, HEIGHT))
}

struct Screen {
    page: usize,
    tick: u32,
}

impl Screen {
    fn draw(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let style = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::Surface0));
        let accent = style.fg(cx.token(Token::Accent));
        Block::default().style(style).render(area, cx);
        Paragraph::new(format!("CHART  /  {}", CASES[self.page]))
            .style(accent)
            .render(Rect::new(3, 1, 94, 1), cx);
        Paragraph::new("← → change chart    u advance sample    q quit")
            .style(style)
            .render(Rect::new(3, 30, 94, 1), cx);
        if self.page == 3 {
            Paragraph::new("Antialiased pixels · native bars, axes and labels")
                .style(style)
                .render(Rect::new(3, 3, 94, 1), cx);
            ChartBoard::new(cx, self.tick).histogram(Rect::new(2, 5, 96, 24), cx);
        } else {
            for index in 0..2_u16 {
                let left = 2 + index * 49;
                Paragraph::new(if index == 0 { "Braille" } else { "Pixels" })
                    .style(accent)
                    .render(Rect::new(left + 1, 3, 44, 1), cx);
                let board = ChartBoard::new(cx, self.tick)
                    .pixels(index == 1)
                    .braille(true);
                let plot = Rect::new(left, 5, 47, 24);
                match self.page {
                    0 => board.throughput(plot, cx),
                    1 => board.saturation(plot, cx),
                    _ => board.scatter(plot, cx),
                }
            }
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
                KeyCode::Right => self.page = (self.page + 1) % CASES.len(),
                KeyCode::Left => self.page = (self.page + CASES.len() - 1) % CASES.len(),
                KeyCode::Char('u') => self.tick = self.tick.wrapping_add(5),
                _ => {}
            }
        }
        ControlFlow::Continue
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        self.draw(area, &mut RenderCx::new(grid, clusters, &theme));
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
    use lunatui_core::Color;
    use lunatui_image::{ImageRequest, ImageSink, ImageSource, PixelSize, PixelTarget};

    #[derive(Default)]
    struct Capture {
        images: Vec<(ImageSource, Rect)>,
    }

    impl ImageSink for Capture {
        fn supports_pixels(&self) -> bool {
            true
        }

        fn pixel_target(&self, area: Rect) -> Option<PixelTarget> {
            PixelSize::new(u32::from(area.width) * 13, u32::from(area.height) * 25)
                .map(|size| PixelTarget::new(size, true, Some([11, 22, 33])))
        }

        fn submit(&mut self, _: ImageRequest<'_>) -> bool {
            false
        }

        fn submit_raster(&mut self, source: &ImageSource, area: Rect, _: i32) -> bool {
            self.images.push((source.clone(), area));
            true
        }
    }

    fn render(screen: &Screen, capture: &mut Capture) -> (Grid, ClusterTable) {
        let mut theme = Theme::builtin(Variant::Dark);
        theme.set_token(Token::Surface0, Color::Rgb(11, 22, 33));
        theme.set_token(Token::Accent, Color::Rgb(255, 0, 0));
        theme.set_token(Token::FocusRing, Color::Rgb(0, 255, 0));
        let mut grid = Grid::new(WIDTH, HEIGHT);
        let mut clusters = ClusterTable::new();
        screen.draw(
            Rect::new(0, 0, WIDTH, HEIGHT),
            &mut RenderCx::new(&mut grid, &mut clusters, &theme).image_sink(capture),
        );
        (grid, clusters)
    }

    fn pixel(source: &ImageSource, x: usize, y: usize) -> &[u8] {
        let offset = (y * usize::try_from(source.size().width()).unwrap_or(0) + x) * 4;
        &source.rgba8_pixels()[offset..offset + 4]
    }

    #[test]
    fn saturation_to_latency_replaces_the_native_scene_and_moves_swatches_into_the_new_legend()
    -> Result<(), String> {
        let mut screen = Screen { page: 1, tick: 10 };
        let mut capture = Capture::default();
        render(&screen, &mut capture);
        let (saturation, _) = capture.images.pop().ok_or("Saturation raster missing")?;
        assert!(
            (0..100).any(|y| (0..100).any(|x| pixel(&saturation, x, y) != [11, 22, 33, 255])),
            "negative control must contain the old top-left legend markers"
        );
        screen.update(&Event::Key(lunatui_term::KeyEvent::press(KeyCode::Right)));
        let (grid, clusters) = render(&screen, &mut capture);
        assert_eq!(screen.page, 2);
        assert_eq!(
            capture.images.len(),
            1,
            "only the pixel twin submits an image"
        );
        assert!(
            grid.row_display(1, &clusters)
                .contains("CHART  /  Latency scatter")
        );
        let (scatter, area) = capture.images.pop().ok_or("Latency raster missing")?;
        for y in 0..100 {
            for x in 0..100 {
                assert_eq!(
                    pixel(&scatter, x, y),
                    [11, 22, 33, 255],
                    "old lines or loose swatches at {x},{y}"
                );
            }
        }
        for (name, expected) in [("cached", [255, 0, 0, 255]), ("cold", [0, 255, 0, 255])] {
            let (column, row) = (0..HEIGHT)
                .find_map(|row| {
                    let text = grid.row_display(row, &clusters);
                    text.match_indices(name).find_map(|(byte, _)| {
                        let column = text[..byte].chars().count();
                        (column > usize::from(WIDTH) / 2).then_some((column, row))
                    })
                })
                .ok_or_else(|| format!("missing right legend label {name}"))?;
            let x = (column - 2 - usize::from(area.x)) * 13 + 6;
            let y = usize::from(row - area.y) * 25 + 12;
            assert!(x > usize::try_from(scatter.size().width()).unwrap_or(0) / 2);
            assert!(y > usize::try_from(scatter.size().height()).unwrap_or(0) / 2);
            assert_eq!(
                pixel(&scatter, x, y),
                expected,
                "swatch must align two cells before {name}"
            );
        }
        Ok(())
    }
}
