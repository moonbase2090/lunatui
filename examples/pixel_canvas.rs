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

mod canvas_showcase;

const WIDTH: u16 = 120;
const HEIGHT: u16 = 36;

fn main() -> io::Result<()> {
    run(
        Screen {
            scene: 0,
            tick: 54,
            right_pixels: true,
        },
        RunOptions::new(WIDTH, HEIGHT),
    )
}

struct Screen {
    scene: usize,
    tick: u32,
    right_pixels: bool,
}

fn twin_areas(area: Rect) -> Option<[Rect; 2]> {
    if area.width < 20 || area.height < 14 {
        return None;
    }
    let width = (area.width - 5) / 2;
    let height = (area.height - 7).min((width - 2) * 2 / 5 + 3);
    let top = area.y.saturating_add(5 + (area.height - 7 - height) / 2);
    Some([
        Rect::new(area.x.saturating_add(1), top, width, height),
        Rect::new(area.x.saturating_add(width + 3), top, width, height),
    ])
}

impl Screen {
    fn local_tick(&self) -> u32 {
        self.tick % canvas_showcase::SCENES[self.scene].2
    }

    fn draw(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let text = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::Surface0));
        let accent = text.fg(cx.token(Token::FocusRing));
        Block::default().style(text).render(area, cx);
        Paragraph::new(format!(
            "CANVAS  /  {}",
            canvas_showcase::SCENES[self.scene].0
        ))
        .style(accent)
        .render(
            Rect::new(
                area.x.saturating_add(3),
                area.y.saturating_add(1),
                area.width.saturating_sub(6),
                1,
            ),
            cx,
        );
        Paragraph::new("Same scene, sample, dimensions and labels")
            .style(text)
            .render(
                Rect::new(
                    area.x.saturating_add(3),
                    area.y.saturating_add(2),
                    area.width.saturating_sub(6),
                    1,
                ),
                cx,
            );
        Paragraph::new("← → change scene    u advance sample    b toggle right pixels    q quit")
            .style(text)
            .render(
                Rect::new(
                    area.x.saturating_add(3),
                    area.y.saturating_add(area.height.saturating_sub(1)),
                    area.width.saturating_sub(6),
                    1,
                ),
                cx,
            );
        let Some(panes) = twin_areas(area) else {
            return;
        };
        let local = self.local_tick();
        for (index, pane) in panes.into_iter().enumerate() {
            Paragraph::new(if index == 0 {
                "Braille"
            } else if self.right_pixels {
                "Pixels"
            } else {
                "Cells (pixels off)"
            })
            .style(accent)
            .render(
                Rect::new(
                    pane.x.saturating_add(1),
                    pane.y.saturating_sub(2),
                    pane.width.saturating_sub(2),
                    1,
                ),
                cx,
            );
            canvas_showcase::render_scene(
                cx,
                pane,
                self.scene,
                local,
                index == 1 && self.right_pixels,
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
                KeyCode::Right => self.scene = (self.scene + 1) % canvas_showcase::SCENES.len(),
                KeyCode::Left => {
                    self.scene = (self.scene + canvas_showcase::SCENES.len() - 1)
                        % canvas_showcase::SCENES.len();
                }
                KeyCode::Char('u') => self.tick = self.tick.wrapping_add(6),
                KeyCode::Char('b') => self.right_pixels = !self.right_pixels,
                _ => {}
            }
        }
        ControlFlow::Continue
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        let before: u32 = canvas_showcase::SCENES[..self.scene]
            .iter()
            .map(|scene| scene.2)
            .sum();
        canvas_showcase::render(grid, clusters, &theme, area, before + self.local_tick());
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
    use lunatui_core::text_display_width;
    use lunatui_image::{ImageRequest, ImageSink, ImageSource, PixelSize, PixelTarget};

    use super::*;

    struct Capture {
        cell_height: u32,
        images: Vec<(ImageSource, Rect, i32)>,
    }

    impl ImageSink for Capture {
        fn supports_pixels(&self) -> bool {
            true
        }

        fn pixel_target(&self, area: Rect) -> Option<PixelTarget> {
            PixelSize::new(
                u32::from(area.width) * 13,
                u32::from(area.height) * self.cell_height,
            )
            .map(|size| PixelTarget::new(size, true, Some([24, 27, 33])))
        }

        fn submit(&mut self, _: ImageRequest<'_>) -> bool {
            false
        }

        fn submit_raster(&mut self, source: &ImageSource, area: Rect, depth: i32) -> bool {
            assert_eq!(source.size().width(), u32::from(area.width) * 13);
            assert_eq!(
                source.size().height(),
                u32::from(area.height) * self.cell_height
            );
            self.images.push((source.clone(), area, depth));
            true
        }
    }

    #[test]
    fn every_scene_accepts_native_rasters_at_the_documented_terminal_size_and_keeps_text_labels() {
        let labels = ["ratio", "Flights", "50%", "10:08:", "200 boids", "ms"];
        for cell_height in [25, 26] {
            for (scene, label) in labels.into_iter().enumerate() {
                let mut grid = Grid::new(WIDTH, HEIGHT);
                let mut clusters = ClusterTable::new();
                let theme = Theme::builtin(Variant::Dark);
                let mut capture = Capture {
                    cell_height,
                    images: Vec::new(),
                };
                Screen {
                    scene,
                    tick: 18,
                    right_pixels: true,
                }
                .draw(
                    Rect::new(0, 0, WIDTH, HEIGHT),
                    &mut RenderCx::new(&mut grid, &mut clusters, &theme).image_sink(&mut capture),
                );
                assert_eq!(
                    capture.images.len(),
                    if scene == 2 { 3 } else { 1 },
                    "{}",
                    canvas_showcase::SCENES[scene].0
                );
                let rows: Vec<_> = (0..HEIGHT)
                    .map(|row| grid.row_display(row, &clusters))
                    .collect();
                for (image, image_area, depth) in &capture.images {
                    assert_eq!(*depth, -1, "labels require Kitty underlay");
                    let visible_label =
                        (image_area.y..image_area.y + image_area.height).any(|row| {
                            let text = &rows[usize::from(row)];
                            text.match_indices(label).any(|(offset, _)| {
                                let column = text_display_width(&text[..offset]);
                                column >= usize::from(image_area.x)
                                    && column + text_display_width(label)
                                        <= usize::from(image_area.x + image_area.width)
                            })
                        });
                    assert!(visible_label, "missing native label {label}: {rows:?}");
                    let rgba = image.rgba8_pixels().as_chunks::<4>().0;
                    assert!(rgba.iter().all(|pixel| pixel[3] == 255));
                    assert!(
                        rgba.iter().any(|pixel| pixel != &rgba[0]),
                        "native scene is blank"
                    );
                }
            }
        }
    }

    #[test]
    fn twins_keep_equal_dimensions_through_resize_and_leave_the_odd_column_in_a_gutter() {
        for (width, height, pane_width, pane_height) in
            [(120, 42, 57, 25), (112, 40, 53, 23), (121, 43, 58, 25)]
        {
            let panes =
                twin_areas(Rect::new(0, 0, width, height)).unwrap_or([Rect::new(0, 0, 0, 0); 2]);
            assert_eq!((panes[0].width, panes[0].height), (pane_width, pane_height));
            assert_eq!((panes[1].width, panes[1].height), (pane_width, pane_height));
            assert!(panes[0].x + panes[0].width < panes[1].x);
            assert!(panes[1].x + panes[1].width <= width);
        }
    }

    #[test]
    fn toggling_the_right_pane_omits_native_images_and_restores_the_same_sample()
    -> Result<(), Box<dyn std::error::Error>> {
        let area = Rect::new(0, 0, WIDTH, HEIGHT);
        let panes = twin_areas(area).ok_or("missing twin layout")?;
        let theme = Theme::builtin(Variant::Dark);
        let mut screen = Screen {
            scene: 2,
            tick: 18,
            right_pixels: true,
        };
        let paint = |screen: &Screen, capture: &mut Capture| {
            let mut grid = Grid::new(WIDTH, HEIGHT);
            let mut clusters = ClusterTable::new();
            screen.draw(
                area,
                &mut RenderCx::new(&mut grid, &mut clusters, &theme).image_sink(capture),
            );
            grid
        };
        let mut before = Capture {
            cell_height: 25,
            images: Vec::new(),
        };
        let first = paint(&screen, &mut before);
        assert_eq!(before.images.len(), 3);
        let toggle = Event::Key(lunatui_term::KeyEvent::press(KeyCode::Char('b')));
        assert_eq!(screen.update(&toggle), ControlFlow::Continue);
        let mut off = Capture {
            cell_height: 25,
            images: Vec::new(),
        };
        let cells = paint(&screen, &mut off);
        assert_eq!(off.images, []);
        assert_eq!((screen.scene, screen.tick), (2, 18));
        let mut expected = Grid::new(WIDTH, HEIGHT);
        let mut clusters = ClusterTable::new();
        canvas_showcase::render_scene(
            &mut RenderCx::new(&mut expected, &mut clusters, &theme),
            panes[1],
            2,
            18,
            false,
        );
        for (index, pane) in panes.into_iter().enumerate() {
            for row in pane.y..pane.y + pane.height {
                for column in pane.x..pane.x + pane.width {
                    assert_eq!(
                        cells.cell_bits(column, row),
                        if index == 0 {
                            first.cell_bits(column, row)
                        } else {
                            expected.cell_bits(column, row)
                        }
                    );
                }
            }
        }
        assert_eq!(screen.update(&toggle), ControlFlow::Continue);
        let mut restored = Capture {
            cell_height: 25,
            images: Vec::new(),
        };
        paint(&screen, &mut restored);
        assert_eq!(restored.images.len(), 3);
        assert_eq!(restored.images, before.images);
        assert_eq!((screen.scene, screen.tick), (2, 18));
        Ok(())
    }
}
