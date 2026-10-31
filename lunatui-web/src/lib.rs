// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_runtime::{App, ControlFlow};
use lunatui_term::{ColorMode, Encoder, Parser};
use wasm_bindgen::prelude::{JsError, wasm_bindgen};

/// Form quickstart source shared with the native example.
#[path = "../../examples/templates/form.rs"]
pub mod form;
/// List quickstart source shared with the native example.
#[path = "../../examples/templates/list.rs"]
pub mod list;
/// Minimal quickstart source shared with the native example.
#[path = "../../examples/templates/minimal.rs"]
pub mod minimal;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Template {
    Minimal,
    List,
    Form,
}

impl Template {
    fn parse(name: &str) -> Result<Self, &'static str> {
        match name {
            "minimal" => Ok(Self::Minimal),
            "list" => Ok(Self::List),
            "form" => Ok(Self::Form),
            _ => Err("unknown template; expected minimal, list, or form"),
        }
    }

    fn app(self) -> Box<dyn App> {
        match self {
            Self::Minimal => Box::new(minimal::app()),
            Self::List => Box::new(list::app()),
            Self::Form => Box::new(form::app()),
        }
    }
}

enum State {
    Running(Box<dyn App>),
    Exited,
}

/// One browser-side app session with its input parser and previous frame.
#[wasm_bindgen]
pub struct Session {
    state: State,
    parser: Parser,
    previous: Grid,
    encoder: Encoder,
    area: Rect,
}

#[wasm_bindgen]
impl Session {
    /// Starts one of the quickstart templates at the requested terminal size.
    ///
    /// # Errors
    ///
    /// Returns an error when `template` is not `minimal`, `list`, or `form`.
    #[wasm_bindgen(constructor)]
    pub fn new(template: &str, columns: u16, rows: u16) -> Result<Self, JsError> {
        let template = Template::parse(template).map_err(JsError::new)?;
        Ok(Self::from_template(template, columns, rows))
    }

    /// Decodes terminal input, updates the app, and returns its ANSI frame diff.
    #[must_use]
    pub fn input(&mut self, bytes: &[u8]) -> Vec<u8> {
        if !self.is_running() {
            return Vec::new();
        }

        let mut events = self.parser.push(bytes);
        events.extend(self.parser.flush());
        if events.is_empty() {
            return Vec::new();
        }

        let State::Running(app) = &mut self.state else {
            return Vec::new();
        };
        let exited = events
            .iter()
            .any(|event| app.update(event) == ControlFlow::Break);
        if exited {
            self.state = State::Exited;
            return Vec::new();
        }
        self.frame()
    }

    /// Renders a timer-driven frame at `milliseconds` since the page time origin.
    ///
    /// The first tick paints the initial frame. The phase-one templates do not
    /// animate, but the timestamp keeps browser time at this boundary when an
    /// animated template is added later.
    #[must_use]
    pub fn tick(&mut self, milliseconds: f64) -> Vec<u8> {
        if !milliseconds.is_finite() {
            return Vec::new();
        }
        self.frame()
    }

    /// Changes the terminal size and returns a complete ANSI frame for it.
    #[must_use]
    pub fn resize(&mut self, columns: u16, rows: u16) -> Vec<u8> {
        if self.area.width == columns && self.area.height == rows {
            return Vec::new();
        }
        self.area = Rect::new(0, 0, columns, rows);
        self.previous = Grid::new(columns, rows);
        self.frame()
    }

    /// Reports whether the template is still accepting input.
    #[must_use]
    pub fn is_running(&self) -> bool {
        matches!(self.state, State::Running(_))
    }
}

impl Session {
    fn from_template(template: Template, columns: u16, rows: u16) -> Self {
        Self {
            state: State::Running(template.app()),
            parser: Parser::new(),
            previous: Grid::new(columns, rows),
            encoder: Encoder::new(),
            area: Rect::new(0, 0, columns, rows),
        }
    }

    fn frame(&mut self) -> Vec<u8> {
        let State::Running(app) = &mut self.state else {
            return Vec::new();
        };
        let mut next = Grid::new(self.area.width, self.area.height);
        let mut clusters = ClusterTable::new();
        app.render(&mut next, self.area, &mut clusters);
        self.encoder
            .set_token_palette(app.token_palette().unwrap_or_default().to_vec());
        let bytes = self
            .encoder
            .encode_diff(&self.previous, &next, &clusters, ColorMode::TrueColor)
            .to_vec();
        self.previous = next;
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::{Session, Template};
    use lunatui_core::{ClusterTable, Grid, Rect};
    use lunatui_runtime::App;
    use lunatui_term::{ColorMode, Encoder, Parser};

    fn render(app: &mut dyn App, columns: u16, rows: u16) -> (Grid, ClusterTable) {
        let mut grid = Grid::new(columns, rows);
        let mut clusters = ClusterTable::new();
        app.render(&mut grid, Rect::new(0, 0, columns, rows), &mut clusters);
        (grid, clusters)
    }

    fn encode(previous: &Grid, next: &Grid, clusters: &ClusterTable) -> Vec<u8> {
        Encoder::new()
            .encode_diff(previous, next, clusters, ColorMode::TrueColor)
            .to_vec()
    }

    fn parsed(bytes: &[u8]) -> Vec<lunatui_term::Event> {
        let mut parser = Parser::new();
        let mut events = parser.push(bytes);
        events.extend(parser.flush());
        events
    }

    #[test]
    fn native_frames_match_the_template_renderer() {
        for (template, columns, rows, expected_text) in [
            (Template::Minimal, 48, 10, "Hello. Press q to quit."),
            (Template::List, 42, 14, "Gauge"),
            (Template::Form, 56, 12, "Ada"),
        ] {
            let mut reference = template.app();
            let (grid, clusters) = render(reference.as_mut(), columns, rows);
            assert!(grid.display_text(&clusters).contains(expected_text));
            let expected = encode(&Grid::new(columns, rows), &grid, &clusters);

            let mut session = Session::from_template(template, columns, rows);
            assert_eq!(session.tick(0.0), expected);
        }
    }

    #[test]
    fn terminal_input_matches_direct_app_updates() {
        for (template, input, columns, rows) in [
            (Template::List, b"\x1b[B".as_slice(), 42, 14),
            (Template::Form, b"Z".as_slice(), 56, 12),
        ] {
            let mut reference = template.app();
            let (previous, _) = render(reference.as_mut(), columns, rows);
            for event in parsed(input) {
                let _ = reference.update(&event);
            }
            let (next, clusters) = render(reference.as_mut(), columns, rows);
            let expected = encode(&previous, &next, &clusters);

            let mut session = Session::from_template(template, columns, rows);
            let _ = session.tick(0.0);
            assert_eq!(session.input(input), expected);
        }
    }

    #[test]
    fn resize_renders_from_a_blank_frame() {
        let mut reference = Template::Minimal.app();
        let (next, clusters) = render(reference.as_mut(), 60, 12);
        let expected = encode(&Grid::new(60, 12), &next, &clusters);

        let mut session = Session::from_template(Template::Minimal, 48, 10);
        let _ = session.tick(0.0);
        assert_eq!(session.resize(60, 12), expected);
    }

    #[test]
    fn quit_stops_the_session() {
        let mut session = Session::from_template(Template::Minimal, 48, 10);
        let _ = session.tick(0.0);
        assert!(session.is_running());
        assert_eq!(session.input(b"q"), Vec::<u8>::new());
        assert!(!session.is_running());
        assert_eq!(session.input(b"x"), Vec::<u8>::new());
    }

    #[test]
    fn unknown_template_is_rejected() {
        assert_eq!(
            Template::parse("dashboard"),
            Err("unknown template; expected minimal, list, or form")
        );
    }
}
