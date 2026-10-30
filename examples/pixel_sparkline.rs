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

mod spark_scene;

const WIDTH: u16 = 200;
const HEIGHT: u16 = 44;
const CASES: [(&str, u32); 6] = [
    ("CPU", 0),
    ("Network", 48),
    ("Requests", 96),
    ("Errors", 154),
    ("Cache", 192),
    ("Connections", 216),
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
        Block::default().style(style).render(area, cx);
        Paragraph::new(format!("SPARKLINE  /  {}", CASES[self.page].0))
            .style(accent)
            .render(Rect::new(3, 1, 194, 1), cx);
        Paragraph::new("← → select scene    u advance sample    b toggle right pane    q quit")
            .style(style)
            .render(Rect::new(3, 42, 194, 1), cx);
        let tick = CASES[self.page].1.wrapping_add(self.sample);
        for index in 0..2_u16 {
            let left = 2 + index * 99;
            let pixels = index == 1 && self.pixels;
            Paragraph::new(if pixels { "Pixels" } else { "Cells" })
                .style(accent)
                .render(Rect::new(left + 1, 3, 94, 1), cx);
            spark_scene::render_scene(Rect::new(left, 5, 97, 36), tick, pixels, cx);
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
                KeyCode::Char('u') => self.sample = self.sample.wrapping_add(5),
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
    use lunatui_core::Color;
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
            .map(|size| PixelTarget::new(size, false, Some([11, 22, 33])))
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
        let mut theme = Theme::builtin(Variant::Dark);
        theme.set_token(Token::Surface0, Color::Rgb(11, 22, 33));
        theme.set_token(Token::Fail, Color::Rgb(255, 0, 0));
        theme.set_token(Token::FocusRing, Color::Rgb(0, 255, 0));
        let mut grid = Grid::new(WIDTH, HEIGHT);
        let mut clusters = ClusterTable::new();
        screen.draw(
            Rect::new(0, 0, WIDTH, HEIGHT),
            &mut RenderCx::new(&mut grid, &mut clusters, &theme).image_sink(capture),
        );
        (grid, clusters)
    }

    fn press(screen: &mut Screen, code: KeyCode) {
        screen.update(&Event::Key(lunatui_term::KeyEvent::press(code)));
    }

    #[test]
    fn all_service_scenes_render_eleven_native_sparklines_and_all_ten_lanes() {
        for height in [25, 26] {
            let mut screen = Screen {
                page: 0,
                sample: 0,
                pixels: true,
            };
            for (name, _) in CASES {
                let mut capture = Capture {
                    height,
                    images: Vec::new(),
                };
                let (grid, clusters) = draw(&screen, &mut capture);
                assert_eq!(capture.images.len(), 11, "{name} at {height}px");
                assert!(capture.images.iter().all(|(_, area)| area.x >= 101
                    && area.x + area.width <= 198
                    && area.y >= 5
                    && area.y + area.height <= 41));
                let text = grid.display_text(&clusters);
                for lane in [
                    "cpu", "mem", "net in", "net out", "req/s", "p99", "errors", "disk", "cache",
                    "conns",
                ] {
                    assert!(
                        text.matches(lane).count() >= 2,
                        "missing lane {lane} on {name}"
                    );
                }
                if name == "Errors" {
                    assert!(text.contains("threshold 8%  over"));
                    assert!(
                        capture.images[6]
                            .0
                            .rgba8_pixels()
                            .as_chunks::<4>()
                            .0
                            .contains(&[255, 0, 0, 255])
                    );
                    assert!(
                        capture.images[10]
                            .0
                            .rgba8_pixels()
                            .as_chunks::<4>()
                            .0
                            .contains(&[255, 0, 0, 255])
                    );
                }
                press(&mut screen, KeyCode::Right);
            }
        }
    }

    #[test]
    fn switching_to_cells_produces_complete_identical_twins_and_restores_native_scenes() {
        for (page, (name, _)) in CASES.iter().enumerate() {
            let mut screen = Screen {
                page,
                sample: 0,
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
            for y in 5..41 {
                for x in 0..97 {
                    assert_eq!(
                        grid.cell_bits(2 + x, y),
                        grid.cell_bits(101 + x, y),
                        "{name} at {x},{y}"
                    );
                }
            }
            press(&mut screen, KeyCode::Char('b'));
            draw(&screen, &mut capture);
            assert_eq!(capture.images.len(), 11);
        }
    }

    #[test]
    fn sample_key_changes_both_original_service_data_views() {
        let mut screen = Screen {
            page: 0,
            sample: 0,
            pixels: true,
        };
        let mut before = Capture {
            height: 25,
            images: Vec::new(),
        };
        let (first, _) = draw(&screen, &mut before);
        press(&mut screen, KeyCode::Char('u'));
        assert_eq!(screen.sample, 5);
        let mut after = Capture {
            height: 25,
            images: Vec::new(),
        };
        let (second, _) = draw(&screen, &mut after);
        assert_ne!(
            before.images[0].0.rgba8_pixels(),
            after.images[0].0.rgba8_pixels()
        );
        assert_ne!(first.row_bits(9), second.row_bits(9));
    }
}
