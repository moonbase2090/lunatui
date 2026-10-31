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

mod deploy_board;

const WIDTH: u16 = 200;
const HEIGHT: u16 = 44;
const CASES: [(&str, u64); 6] = [
    ("Start", 0),
    ("Canary", 100),
    ("Quarter", 1900),
    ("Half", 3700),
    ("Ramp", 6300),
    ("Almost", 7900),
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
    sample: u64,
    pixels: bool,
}

impl Screen {
    fn draw(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let style = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::Surface0));
        let accent = style.fg(cx.token(Token::Accent));
        Block::new().style(style).render(area, cx);
        Paragraph::new(format!("GAUGE  /  {}", CASES[self.page].0))
            .style(accent)
            .render(Rect::new(3, 1, 194, 1), cx);
        Paragraph::new("← → select stage    u advance sample    b toggle right pane    q quit")
            .style(style)
            .render(Rect::new(3, 42, 194, 1), cx);
        let elapsed = CASES[self.page].1.wrapping_add(self.sample);
        for index in 0..2_u16 {
            let left = 2 + index * 99;
            let pixels = index == 1 && self.pixels;
            Paragraph::new(if pixels { "Pixels" } else { "Cells" })
                .style(accent)
                .render(Rect::new(left + 1, 3, 94, 1), cx);
            deploy_board::draw(Rect::new(left, 5, 97, 36), elapsed, pixels, cx);
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
                KeyCode::Char('u') => self.sample = self.sample.wrapping_add(100),
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
        fn submit_raster(&mut self, source: &ImageSource, area: Rect, depth: i32) -> bool {
            assert_eq!(depth, 0);
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
    #[test]
    fn all_stages_keep_the_complete_shared_board_and_only_the_right_gauge_uses_pixels() {
        for height in [25, 26] {
            for page in 0..CASES.len() {
                let screen = Screen {
                    page,
                    sample: 0,
                    pixels: true,
                };
                let mut images = Capture {
                    height,
                    images: Vec::new(),
                };
                let (grid, _) = draw(&screen, &mut images);
                for left in [2, 101] {
                    let mut text = String::new();
                    for y in 5..41 {
                        for x in left..left + 97 {
                            if let Some(lunatui_core::DecodedGlyph::Inline(ch)) =
                                grid.get(x, y).map(|cell| cell.glyph().decode())
                            {
                                text.push(ch);
                            }
                        }
                        text.push('\n');
                    }
                    for label in [
                        "production deploy",
                        "compiling workspace",
                        "418 passed",
                        "canary",
                        "worker",
                        "migrate",
                        "monitor",
                        "rollout",
                        "no incidents",
                    ] {
                        assert!(text.contains(label), "{label}, page {page}, left {left}");
                    }
                }
                assert_eq!(images.images.len(), usize::from(page != 0));
                for (_, area) in images.images {
                    assert_eq!(area, Rect::new(103, 37, 93, 1));
                }
            }
        }
    }
    #[test]
    fn fallback_twins_match_in_every_cell_and_toggling_back_restores_native_fill() {
        for page in 0..CASES.len() {
            let mut screen = Screen {
                page,
                sample: 0,
                pixels: false,
            };
            let mut images = Capture {
                height: 26,
                images: Vec::new(),
            };
            let (grid, _) = draw(&screen, &mut images);
            assert_eq!(images.images.len(), 0);
            for y in 5..41 {
                for x in 0..97 {
                    assert_eq!(grid.get(x + 2, y), grid.get(x + 101, y), "{page} {x},{y}");
                }
            }
            screen.pixels = true;
            draw(&screen, &mut images);
            assert_eq!(images.images.len(), usize::from(page != 0));
        }
    }
    #[test]
    fn advancing_sample_changes_native_geometry_on_both_sides_of_the_comparison() {
        let mut screen = Screen {
            page: 1,
            sample: 0,
            pixels: true,
        };
        let mut first = Capture {
            height: 25,
            images: Vec::new(),
        };
        let (before, _) = draw(&screen, &mut first);
        screen.sample = 100;
        let mut second = Capture {
            height: 25,
            images: Vec::new(),
        };
        let (after, _) = draw(&screen, &mut second);
        assert_ne!(before.row_bits(37), after.row_bits(37));
        assert_ne!(
            first.images[0].0.rgba8_pixels(),
            second.images[0].0.rgba8_pixels()
        );
    }
}
