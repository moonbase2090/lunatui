// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::io;
use std::time::Instant;

use lunatui::{App, ControlFlow, RunOptions, is_quit, run};
use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_runtime::RenderFrame;
use lunatui_term::{Event, KeyCode};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{ImageId, Paragraph, PixelPieChart, RenderCx, Widget};

mod pie_scene;

fn main() -> io::Result<()> {
    let mut compare = false;
    let mut hold = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--compare" => compare = true,
            "--hold" => {
                hold = Some(
                    args.next()
                        .ok_or_else(|| io::Error::other("--hold requires a tick"))?
                        .parse::<u32>()
                        .map_err(|_| io::Error::other("--hold requires an integer tick"))?,
                );
            }
            _ => {
                return Err(io::Error::other(
                    "usage: pixel_pie_chart [--compare] [--hold TICK]",
                ));
            }
        }
    }
    let id = ImageId::new(41).ok_or_else(|| io::Error::other("invalid chart identity"))?;
    run(
        Screen {
            pie: PixelPieChart::new(
                id,
                pie_scene::specification(&pie_scene::ClusterFrame::at(0)),
            ),
            started: Instant::now(),
            hold,
            selected: None,
            visible: true,
            pixels: true,
            compare,
        },
        if compare {
            RunOptions::new(248, 44)
        } else {
            RunOptions::new(120, 36)
        },
    )
}

struct Screen {
    pie: PixelPieChart,
    started: Instant,
    hold: Option<u32>,
    selected: Option<usize>,
    visible: bool,
    pixels: bool,
    compare: bool,
}

impl Screen {
    fn current_frame(&self) -> pie_scene::ClusterFrame {
        let tick = self.hold.unwrap_or_else(|| {
            u32::try_from(self.started.elapsed().as_millis().saturating_mul(30) / 1000)
                .unwrap_or(u32::MAX)
        });
        let mut frame = pie_scene::ClusterFrame::at(tick);
        if let Some(selected) = self.selected {
            frame.selected = Some(selected);
        }
        frame
    }

    fn draw(&mut self, area: Rect, cx: &mut RenderCx<'_>) {
        let frame = self.current_frame();
        let tick = frame.tick;
        if !self.compare {
            let display = if !self.visible {
                pie_scene::Display::Hidden
            } else if self.pixels {
                pie_scene::Display::Native(&mut self.pie)
            } else {
                pie_scene::Display::Cells
            };
            pie_scene::draw(area, &frame, display, cx);
            return;
        }
        let style = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::Surface0));
        let accent = style.fg(cx.token(Token::Accent));
        let blank = vec![" ".repeat(usize::from(area.width)); usize::from(area.height)].join("\n");
        Paragraph::new(blank).style(style).render(area, cx);
        Paragraph::new(format!("PIE CHART  /  Cluster overview  /  tick {tick}"))
            .style(accent)
            .render(Rect::new(area.x + 3, area.y + 1, 242, 1), cx);
        for index in 0..2_u16 {
            let left = area.x + 2 + index * 124;
            let pixels = index == 1 && self.pixels;
            Paragraph::new(if pixels { "Pixels" } else { "Cells" })
                .style(accent)
                .render(Rect::new(left + 1, area.y + 3, 118, 1), cx);
            let display = if !self.visible {
                pie_scene::Display::Hidden
            } else if pixels {
                pie_scene::Display::Native(&mut self.pie)
            } else {
                pie_scene::Display::Cells
            };
            pie_scene::draw(Rect::new(left, area.y + 5, 120, 36), &frame, display, cx);
        }
        Paragraph::new("← → select region    u advance held tick    b toggle right pane    Space hide / show    r reset    q quit")
            .style(style).render(Rect::new(area.x + 3, area.y + 42, 242, 1), cx);
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
                    self.selected = Some(
                        self.current_frame()
                            .selected
                            .map_or(0, |index| (index + 1) % 5),
                    );
                }
                KeyCode::Left => {
                    self.selected = Some(
                        self.current_frame()
                            .selected
                            .map_or(4, |index| (index + 4) % 5),
                    );
                }
                KeyCode::Char(' ') => self.visible = !self.visible,
                KeyCode::Char('b') => self.pixels = !self.pixels,
                KeyCode::Char('u') => {
                    if let Some(tick) = &mut self.hold {
                        *tick = tick.wrapping_add(1);
                    }
                }
                KeyCode::Char('r') => {
                    self.selected = None;
                    self.started = Instant::now();
                }
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
    fn needs_animation_frames(&self) -> bool {
        self.hold.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lunatui_image::{ImageRequest, ImageSink, ImageSource, PixelSize, PixelTarget};

    #[derive(Default)]
    struct Capture {
        images: Vec<(ImageSource, Rect)>,
        decline: bool,
    }
    impl ImageSink for Capture {
        fn supports_pixels(&self) -> bool {
            true
        }
        fn pixel_target(&self, area: Rect) -> Option<PixelTarget> {
            PixelSize::new(u32::from(area.width) * 4, u32::from(area.height) * 8)
                .map(|size| PixelTarget::new(size, true, Some([0, 0, 0])))
        }
        fn submit(&mut self, request: ImageRequest<'_>) -> bool {
            if self.decline {
                return false;
            }
            self.images.push((request.source().clone(), request.area()));
            true
        }
        fn submit_raster(&mut self, source: &ImageSource, area: Rect, _: i32) -> bool {
            if self.decline {
                return false;
            }
            self.images.push((source.clone(), area));
            true
        }
    }
    fn screen() -> Result<Screen, Box<dyn std::error::Error>> {
        Ok(Screen {
            pie: PixelPieChart::new(
                ImageId::new(41).ok_or("fixture identity")?,
                pie_scene::specification(&pie_scene::ClusterFrame::at(96)),
            ),
            started: Instant::now(),
            hold: Some(96),
            selected: None,
            visible: true,
            pixels: true,
            compare: true,
        })
    }
    fn frame(
        screen: &mut Screen,
        grid: &mut Grid,
        clusters: &mut ClusterTable,
        decline: bool,
    ) -> Capture {
        let mut capture = Capture {
            decline,
            ..Capture::default()
        };
        screen.draw(
            Rect::new(0, 0, 248, 44),
            &mut RenderCx::new(grid, clusters, &Theme::builtin(Variant::Dark))
                .image_sink(&mut capture),
        );
        capture
    }
    #[test]
    fn full_cluster_twins_share_text_and_only_native_graphs_submit_images()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut screen = screen()?;
        let mut grid = Grid::new(248, 44);
        let mut clusters = ClusterTable::new();
        let capture = frame(&mut screen, &mut grid, &mut clusters, false);
        assert_eq!(
            capture.images.len(),
            3,
            "pie, native legend and native trend"
        );
        for (_, area) in &capture.images {
            assert!(area.x >= 126 && area.x + area.width <= 246);
            assert!(area.y >= 5 && area.y + area.height <= 41);
        }
        for text in [
            "Cluster overview",
            "region       share",
            "eu-west",
            "total req/s",
            "p95",
        ] {
            let all = (5..41)
                .map(|row| grid.row_display(row, &clusters))
                .collect::<Vec<_>>()
                .join("\n");
            assert_eq!(
                all.matches(text).count(),
                if text == "eu-west" {
                    6
                } else if text == "p95" {
                    4
                } else {
                    2
                },
                "{text}"
            );
        }
        for y in 5..41 {
            for x in 0..120 {
                let right_x = 126 + x;
                let native = capture.images.iter().any(|(_, area)| {
                    right_x >= area.x
                        && right_x < area.x + area.width
                        && y >= area.y
                        && y < area.y + area.height
                });
                if !native {
                    assert_eq!(grid.get(2 + x, y), grid.get(right_x, y), "{x},{y}");
                }
            }
        }
        Ok(())
    }
    #[test]
    fn native_cells_hidden_and_restored_preserve_full_scene_and_image_sources()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut screen = screen()?;
        let mut grid = Grid::new(248, 44);
        let mut clusters = ClusterTable::new();
        let before = frame(&mut screen, &mut grid, &mut clusters, false);
        screen.pixels = false;
        assert_eq!(
            frame(&mut screen, &mut grid, &mut clusters, false).images,
            Vec::new()
        );
        for y in 5..41 {
            for x in 0..120 {
                assert_eq!(grid.get(2 + x, y), grid.get(126 + x, y));
            }
        }
        screen.visible = false;
        screen.pixels = true;
        assert_eq!(
            frame(&mut screen, &mut grid, &mut clusters, false).images,
            Vec::new()
        );
        assert_eq!(
            grid.row_display(5, &clusters)
                .matches("Cluster overview")
                .count(),
            2
        );
        screen.visible = true;
        let after = frame(&mut screen, &mut grid, &mut clusters, false);
        assert_eq!(after.images.len(), 3);
        assert_eq!(after.images, before.images);
        let rejected = frame(&mut screen, &mut grid, &mut clusters, true);
        assert_eq!(rejected.images, Vec::new());
        for y in 5..41 {
            for x in 0..120 {
                assert_eq!(grid.get(2 + x, y), grid.get(126 + x, y));
            }
        }
        Ok(())
    }
    #[test]
    fn real_presenter_caches_and_retires_all_three_native_images()
    -> Result<(), Box<dyn std::error::Error>> {
        use lunatui_term::{Caps, CellPixels, GraphicsProtocol, PixelLayer, Protocols};
        let mut layer = PixelLayer::from_caps(&Caps {
            protocols: Protocols {
                graphics: GraphicsProtocol::Kitty,
                ..Protocols::default()
            },
            cell_pixels: CellPixels::new(4, 8),
            background_rgb: Some([0, 0, 0]),
            ..Caps::default()
        });
        let mut screen = screen()?;
        let mut grid = Grid::new(248, 44);
        let mut clusters = ClusterTable::new();
        let theme = Theme::builtin(Variant::Dark);
        for phase in 0..6 {
            screen.pixels = phase != 2;
            screen.visible = phase != 4;
            layer.begin_frame();
            screen.draw(
                Rect::new(0, 0, 248, 44),
                &mut RenderCx::new(&mut grid, &mut clusters, &theme).image_sink(&mut layer),
            );
            let stream = std::str::from_utf8(layer.finish_frame())?;
            let headers = stream
                .split("\x1b_G")
                .skip(1)
                .map(|command| command.split(';').next().unwrap_or(""))
                .collect::<Vec<_>>();
            let count = |action: &str| {
                headers
                    .iter()
                    .filter(|header| header.split(',').any(|field| field == action))
                    .count()
            };
            if phase == 1 {
                assert_eq!(
                    stream, "",
                    "unchanged native frame must reuse all raster bytes and placements"
                );
            } else if phase == 2 || phase == 4 {
                assert_eq!(
                    count("a=d"),
                    3,
                    "phase {phase} must retire pie, swatches and trend"
                );
                assert_eq!(count("a=p"), 0);
            } else {
                assert_eq!(
                    count("a=t"),
                    3,
                    "phase {phase} must upload every live image"
                );
                assert_eq!(count("a=p"), 3);
                let ids = headers
                    .iter()
                    .filter(|header| header.split(',').any(|field| field == "a=p"))
                    .filter_map(|header| {
                        header.split(',').find_map(|field| field.strip_prefix("i="))
                    })
                    .collect::<std::collections::BTreeSet<_>>();
                assert_eq!(
                    ids.len(),
                    3,
                    "caller pie and presenter legend/trend need distinct wire identities"
                );
            }
        }
        Ok(())
    }
}
