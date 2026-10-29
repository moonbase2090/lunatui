// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::env;
use std::io::{self, Write};

use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_runtime::{App, ControlFlow, RunOptions, is_quit, render_once, run};
use lunatui_term::{Event, KeyCode};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Block, Form, FormField, RenderCx, TextInput, Widget};

const WIDTH: u16 = 56;
const HEIGHT: u16 = 12;

struct Screen {
    name: String,
    role: String,
    focus: usize,
}

impl Screen {
    fn new() -> Self {
        Self {
            name: String::from("Ada"),
            role: String::new(),
            focus: 0,
        }
    }

    fn form(&self) -> Form {
        Form::new([
            FormField::new("Name").required(),
            FormField::new("Role").hint("shown on the card"),
        ])
        .focus(Some(self.focus))
    }

    fn field_mut(&mut self) -> &mut String {
        if self.focus == 0 {
            &mut self.name
        } else {
            &mut self.role
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
                KeyCode::Tab | KeyCode::Down => {
                    if let Some(next) = self.form().step(Some(self.focus), true) {
                        self.focus = next;
                    }
                }
                KeyCode::Up => {
                    if let Some(next) = self.form().step(Some(self.focus), false) {
                        self.focus = next;
                    }
                }
                KeyCode::Backspace => {
                    self.field_mut().pop();
                }
                KeyCode::Char(ch) if ch.is_ascii_graphic() => {
                    let field = self.field_mut();
                    if field.len() < 24 {
                        field.push(ch);
                    }
                }
                _ => {}
            }
        }
        ControlFlow::Continue
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        let mut cx = RenderCx::new(grid, clusters, &theme);
        let text = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::Surface0));
        let accent = Style::new()
            .fg(cx.token(Token::Accent))
            .bg(cx.token(Token::Surface0));
        let block = Block::bordered()
            .title(" account ")
            .style(text)
            .border_style(accent)
            .title_style(accent);
        block.render(area, &mut cx);
        let inner = block.inner(area);
        let form = self.form();
        form.render(inner, &mut cx);
        for field in form.layout(inner) {
            let value = if field.index == 0 {
                self.name.as_str()
            } else {
                self.role.as_str()
            };
            if field.input.width == 0 || field.input.height == 0 {
                continue;
            }
            TextInput::new(value, value.len())
                .show_cursor(field.index == self.focus)
                .style(text)
                .field_style(text)
                .render(field.input, &mut cx);
        }
    }
}

fn main() -> io::Result<()> {
    let mut app = Screen::new();
    if env::args().any(|arg| arg == "--snapshot") {
        let (grid, clusters) = render_once(&mut app, WIDTH, HEIGHT);
        io::stdout().write_all(grid.display_text(&clusters).as_bytes())?;
        return Ok(());
    }
    run(app, RunOptions::new(WIDTH, HEIGHT))
}
