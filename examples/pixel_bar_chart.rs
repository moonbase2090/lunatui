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

mod bar_scene;
use bar_scene::BarBoard;

const WIDTH: u16 = 168;
const HEIGHT: u16 = 36;
const CASES: [&str; 4] = ["Requests", "Regional latency", "Build runs", "Queue states"];

fn main() -> io::Result<()> {
    run(
        Screen {
            page: 0,
            tick: 0,
            pixels: true,
        },
        RunOptions::new(WIDTH, HEIGHT),
    )
}

struct Screen {
    page: usize,
    tick: u32,
    pixels: bool,
}

impl Screen {
    fn draw(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let style = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::Surface0));
        let accent = style.fg(cx.token(Token::Accent));
        Block::default().style(style).render(area, cx);
        Paragraph::new(format!("BAR CHART  /  {}", CASES[self.page]))
            .style(accent)
            .render(Rect::new(3, 1, 162, 1), cx);
        Paragraph::new("← → change chart    u advance sample    b toggle right pane    q quit")
            .style(style)
            .render(Rect::new(3, 34, 162, 1), cx);
        for index in 0..2_u16 {
            let left = 2 + index * 83;
            let pixels = index == 1 && self.pixels;
            Paragraph::new(if pixels { "Pixels" } else { "Cells" })
                .style(accent)
                .render(Rect::new(left + 1, 3, 77, 1), cx);
            let board = BarBoard::new(cx, self.tick).pixels(pixels);
            let plot = Rect::new(left, 5, 81, 27);
            match self.page {
                0 => board.requests(plot, cx),
                1 => board.latency(plot, cx),
                2 => board.runs(plot, cx),
                _ => board.states(plot, cx),
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
                KeyCode::Char('b') => self.pixels = !self.pixels,
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
    use lunatui_image::{ImageRequest, ImageSink, ImageSource, PixelSize, PixelTarget};

    struct Capture {
        height: u32,
        images: Vec<(ImageSource, Rect)>,
    }

    impl ImageSink for Capture {
        fn supports_pixels(&self) -> bool {
            true
        }

        fn pixel_target(&self, area: Rect) -> Option<PixelTarget> {
            PixelSize::new(
                u32::from(area.width) * 13,
                u32::from(area.height) * self.height,
            )
            .map(|size| PixelTarget::new(size, true, Some([0, 0, 0])))
        }

        fn submit(&mut self, _: ImageRequest<'_>) -> bool {
            false
        }

        fn submit_raster(&mut self, source: &ImageSource, area: Rect, _: i32) -> bool {
            self.images.push((source.clone(), area));
            true
        }
    }

    fn draw(screen: &Screen, capture: &mut Capture) -> (Grid, ClusterTable) {
        let mut grid = Grid::new(WIDTH, HEIGHT);
        let mut clusters = ClusterTable::new();
        screen.draw(
            Rect::new(0, 0, WIDTH, HEIGHT),
            &mut RenderCx::new(&mut grid, &mut clusters, &Theme::builtin(Variant::Dark))
                .image_sink(capture),
        );
        (grid, clusters)
    }

    fn press(screen: &mut Screen, code: KeyCode) {
        screen.update(&Event::Key(lunatui_term::KeyEvent::press(code)));
    }

    #[test]
    fn every_shared_scene_accepts_native_pixels_with_realistic_cell_metrics() {
        for height in [25, 26] {
            let mut screen = Screen {
                page: 0,
                tick: 0,
                pixels: true,
            };
            for count in [1, 1, 1, 3] {
                let mut capture = Capture {
                    height,
                    images: Vec::new(),
                };
                let (grid, clusters) = draw(&screen, &mut capture);
                assert_eq!(
                    capture.images.len(),
                    count,
                    "{} at {height}px",
                    CASES[screen.page]
                );
                assert!(
                    capture
                        .images
                        .iter()
                        .all(|(_, area)| area.x >= 85 && area.x + area.width <= 166)
                );
                assert!(grid.row_display(3, &clusters).contains("Pixels"));
                if screen.page == 0 {
                    let labels = grid.row_display(30, &clusters);
                    for name in ["api", "web", "jobs", "edge"] {
                        assert_eq!(
                            labels.matches(name).count(),
                            2,
                            "all twelve bars must fit: {labels}"
                        );
                    }
                }
                press(&mut screen, KeyCode::Right);
            }
        }
    }

    #[test]
    fn cell_toggle_makes_complete_twins_then_restores_pixels_for_every_scene() {
        for (page, name) in CASES.iter().enumerate() {
            let mut screen = Screen {
                page,
                tick: 5,
                pixels: true,
            };
            press(&mut screen, KeyCode::Char('b'));
            let mut capture = Capture {
                height: 25,
                images: Vec::new(),
            };
            let (grid, clusters) = draw(&screen, &mut capture);
            assert_eq!(capture.images, Vec::new());
            assert_eq!(grid.row_display(3, &clusters).matches("Cells").count(), 2);
            for y in 5..32 {
                for x in 0..81 {
                    assert_eq!(
                        grid.cell_bits(2 + x, y),
                        grid.cell_bits(85 + x, y),
                        "{name} at {x},{y}"
                    );
                }
            }
            press(&mut screen, KeyCode::Char('b'));
            draw(&screen, &mut capture);
            assert_eq!(capture.images.len(), if page == 3 { 3 } else { 1 });
        }
    }

    #[test]
    fn sample_key_changes_both_real_scenes_and_keeps_group_labels() {
        let mut screen = Screen {
            page: 0,
            tick: 0,
            pixels: true,
        };
        let mut before = Capture {
            height: 25,
            images: Vec::new(),
        };
        let (first, first_clusters) = draw(&screen, &mut before);
        press(&mut screen, KeyCode::Char('u'));
        assert_eq!(screen.tick, 5);
        let mut after = Capture {
            height: 25,
            images: Vec::new(),
        };
        let (second, second_clusters) = draw(&screen, &mut after);
        assert_ne!(
            before.images[0].0.rgba8_pixels(),
            after.images[0].0.rgba8_pixels()
        );
        assert_ne!(first.row_bits(20), second.row_bits(20));
        assert_eq!(
            first.row_display(30, &first_clusters),
            second.row_display(30, &second_clusters)
        );
    }
}
