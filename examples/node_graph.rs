// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use lunatui::{App, ControlFlow, RunOptions, is_quit, run};
use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_runtime::RenderFrame;
use lunatui_term::{Event, KeyCode};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Paragraph, RenderCx, Widget};
use std::io;
use std::time::Instant;

fn main() -> io::Result<()> {
    run(
        Screen {
            started: Instant::now(),
            offset: 0,
            paused: None,
            pixels: true,
            variant: Variant::Dark,
        },
        RunOptions::new(100, 32),
    )
}

struct Screen {
    started: Instant,
    offset: u32,
    paused: Option<u32>,
    pixels: bool,
    variant: Variant,
}

impl Screen {
    fn tick(&self) -> u32 {
        self.paused.unwrap_or_else(|| {
            (self.offset
                + u32::try_from(self.started.elapsed().as_millis() * 30 / 1000 % 240).unwrap_or(0))
                % 240
        })
    }

    fn draw(&self, area: Rect, cx: &mut RenderCx<'_>) {
        lunatui_bench::widget_suite::node_graph::draw(cx, area, self.tick());
        if area.height > 0 {
            let style = Style::new()
                .fg(cx.token(Token::TextPrimary))
                .bg(cx.token(Token::Surface0));
            Paragraph::new("space pause   ← → step   b pixels/cells   t theme   q quit")
                .style(style)
                .render(
                    Rect::new(area.x, area.y + area.height - 1, area.width, 1),
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
                KeyCode::Char(' ') => {
                    if let Some(tick) = self.paused.take() {
                        self.offset = tick;
                        self.started = Instant::now();
                    } else {
                        self.paused = Some(self.tick());
                    }
                }
                KeyCode::Right | KeyCode::Left => {
                    self.offset =
                        (self.tick() + if key.code == KeyCode::Right { 60 } else { 180 }) % 240;
                    self.started = Instant::now();
                    if self.paused.is_some() {
                        self.paused = Some(self.offset);
                    }
                }
                KeyCode::Char('b') => self.pixels = !self.pixels,
                KeyCode::Char('t') => {
                    self.variant = if self.variant == Variant::Dark {
                        Variant::Light
                    } else {
                        Variant::Dark
                    }
                }
                _ => {}
            }
        }
        ControlFlow::Continue
    }

    fn needs_animation_frames(&self) -> bool {
        self.paused.is_none()
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        self.draw(
            area,
            &mut RenderCx::new(grid, clusters, &Theme::builtin(self.variant)),
        );
    }

    fn render_frame(&mut self, frame: &mut RenderFrame<'_>, area: Rect) {
        let theme = Theme::builtin(self.variant);
        let (grid, clusters, images) = frame.parts();
        let mut cx = RenderCx::new(grid, clusters, &theme);
        if self.pixels {
            cx = cx.image_sink(images);
        }
        self.draw(area, &mut cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lunatui_term::KeyEvent;

    #[test]
    fn keys_step_pause_change_theme_and_switch_presentation() {
        let mut screen = Screen {
            started: Instant::now(),
            offset: 0,
            paused: Some(0),
            pixels: true,
            variant: Variant::Dark,
        };
        screen.update(&Event::Key(KeyEvent::press(KeyCode::Right)));
        assert_eq!(screen.tick(), 60);
        screen.update(&Event::Key(KeyEvent::press(KeyCode::Left)));
        assert_eq!(screen.tick(), 0);
        screen.update(&Event::Key(KeyEvent::press(KeyCode::Char('t'))));
        assert_eq!(screen.variant, Variant::Light);
        screen.update(&Event::Key(KeyEvent::press(KeyCode::Char('b'))));
        assert!(!screen.pixels);
        screen.update(&Event::Key(KeyEvent::press(KeyCode::Char(' '))));
        assert!(screen.needs_animation_frames());
        assert!(matches!(
            screen.update(&Event::Key(KeyEvent::press(KeyCode::Char('q')))),
            ControlFlow::Break
        ));
    }
}
