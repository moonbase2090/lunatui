// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::fmt::Write as _;
use std::time::Duration;

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_term::{
    Event, KeyCode, KeyEvent, Modifiers, MouseButton, MouseEvent, MouseKind, Parser, Scroll,
};

use crate::{Clock, styled_snapshot};

/// What the harness drives: state that takes events and draws itself.
///
/// `lunatui_runtime::App` has the same two methods; implement this trait for
/// a small test type that owns the widget state under test, or for a wrapper
/// around an app.
pub trait Component {
    /// Applies one decoded input event.
    fn update(&mut self, event: &Event);

    /// Draws the current state into `grid` for `area`.
    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable);

    /// Called by [`Harness::advance`] with the virtual time after the step.
    /// Components with no animation or timer ignore it.
    fn tick(&mut self, now: Duration) {
        let _ = now;
    }

    /// The focused index, for [`Harness::assert_focus`]. `None` when nothing
    /// is focused or the component has no focus.
    fn focus(&self) -> Option<usize> {
        None
    }

    /// The current value as text, for [`Harness::assert_value`].
    fn value(&self) -> Option<String> {
        None
    }
}

/// A mounted [`Component`] with a grid, a virtual clock and an event log.
pub struct Harness<C: Component> {
    component: C,
    width: u16,
    height: u16,
    grid: Grid,
    clusters: ClusterTable,
    clock: Clock,
    parser: Parser,
    log: Vec<Event>,
}

impl<C: Component> Harness<C> {
    /// Mounts `component` in a `width` by `height` window and draws it once.
    #[must_use]
    pub fn mount(component: C, width: u16, height: u16) -> Self {
        let mut harness = Self {
            component,
            width,
            height,
            grid: Grid::new(width, height),
            clusters: ClusterTable::new(),
            clock: Clock::new(),
            parser: Parser::new(),
            log: Vec::new(),
        };
        harness.redraw();
        harness
    }

    /// The component under test.
    #[must_use]
    pub fn component(&self) -> &C {
        &self.component
    }

    /// The component under test, mutably, for setup that no event expresses.
    pub fn component_mut(&mut self) -> &mut C {
        &mut self.component
    }

    /// Unmounts and returns the component.
    #[must_use]
    pub fn into_component(self) -> C {
        self.component
    }

    /// The virtual time.
    #[must_use]
    pub fn now(&self) -> Duration {
        self.clock.now()
    }

    /// Every event delivered so far, in order, as the component saw them.
    #[must_use]
    pub fn events(&self) -> &[Event] {
        &self.log
    }

    /// Resizes the window and redraws.
    pub fn resize(&mut self, width: u16, height: u16) -> &mut Self {
        self.width = width;
        self.height = height;
        self.grid = Grid::new(width, height);
        self.redraw();
        self
    }

    /// Injects one key by name; see [`parse_key`] for the spelling.
    ///
    /// # Panics
    ///
    /// Panics when `spec` is not a key name, or the terminal encoding of the
    /// key decodes to no event.
    pub fn key(&mut self, spec: &str) -> &mut Self {
        let parsed = parse_key(spec);
        assert!(
            parsed.is_ok(),
            "bad key {spec:?}: {}",
            parsed.as_ref().err().map_or("", String::as_str)
        );
        if let Ok(event) = parsed {
            self.inject(&Event::Key(event));
        }
        self
    }

    /// Injects several keys: names separated by spaces, for example
    /// `"Down Down Enter"`.
    ///
    /// # Panics
    ///
    /// Panics under the same conditions as [`Harness::key`].
    pub fn keys(&mut self, specs: &str) -> &mut Self {
        for spec in specs.split_whitespace() {
            self.key(spec);
        }
        self
    }

    /// Types `text` one character at a time as plain key presses.
    pub fn type_text(&mut self, text: &str) -> &mut Self {
        for ch in text.chars() {
            self.inject(&Event::Key(KeyEvent::press(KeyCode::Char(ch))));
        }
        self
    }

    /// Presses and releases the left button at cell `(x, y)`.
    pub fn click(&mut self, x: u16, y: u16) -> &mut Self {
        self.click_with(MouseButton::Left, x, y)
    }

    /// Presses and releases `button` at cell `(x, y)`.
    pub fn click_with(&mut self, button: MouseButton, x: u16, y: u16) -> &mut Self {
        self.mouse(MouseKind::Down(button), x, y);
        self.mouse(MouseKind::Up(button), x, y)
    }

    /// Moves the pointer to cell `(x, y)` with no button held.
    pub fn hover(&mut self, x: u16, y: u16) -> &mut Self {
        self.mouse(MouseKind::Move, x, y)
    }

    /// Presses the left button at `from`, drags to `to`, and releases there.
    pub fn drag(&mut self, from: (u16, u16), to: (u16, u16)) -> &mut Self {
        self.mouse(MouseKind::Down(MouseButton::Left), from.0, from.1);
        self.mouse(MouseKind::Drag(MouseButton::Left), to.0, to.1);
        self.mouse(MouseKind::Up(MouseButton::Left), to.0, to.1)
    }

    /// Turns the wheel once over cell `(x, y)`.
    pub fn scroll(&mut self, direction: Scroll, x: u16, y: u16) -> &mut Self {
        self.mouse(MouseKind::Scroll(direction), x, y)
    }

    /// Injects one pointer event at cell `(x, y)`.
    pub fn mouse(&mut self, kind: MouseKind, x: u16, y: u16) -> &mut Self {
        self.inject(&Event::Mouse(MouseEvent::new(kind, x, y)))
    }

    /// Injects `event` through the terminal encoding.
    ///
    /// # Panics
    ///
    /// Panics when the encoded bytes decode to no event, which means a
    /// terminal could not have sent it.
    pub fn inject(&mut self, event: &Event) -> &mut Self {
        let bytes = event.encode();
        let mut decoded = self.parser.push(&bytes);
        decoded.extend(self.parser.flush());
        assert!(
            !decoded.is_empty(),
            "{event:?} encodes to {bytes:?}, which the input parser decodes to no event"
        );
        for event in decoded {
            self.component.update(&event);
            self.log.push(event);
        }
        self.redraw();
        self
    }

    /// Moves the virtual clock forward by `delta`, calls the component's
    /// [`Component::tick`] once with the new time, and redraws. A test that
    /// needs several steps calls this several times.
    pub fn advance(&mut self, delta: Duration) -> &mut Self {
        self.clock.advance(delta);
        self.component.tick(self.clock.now());
        self.redraw();
        self
    }

    /// The drawn rows as text, with trailing blanks trimmed.
    #[must_use]
    pub fn rows(&self) -> Vec<String> {
        (0..self.height)
            .map(|y| {
                self.grid
                    .row_display(y, &self.clusters)
                    .trim_end()
                    .to_owned()
            })
            .collect()
    }

    /// The drawn frame as a styled snapshot: text plus colour and attribute runs.
    #[must_use]
    pub fn styled(&self) -> String {
        styled_snapshot(&self.grid, &self.clusters)
    }

    /// The drawn grid, for assertions on one cell's style.
    #[must_use]
    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    /// Asserts the drawn rows equal `expected`.
    ///
    /// # Panics
    ///
    /// Panics with both frames and the event log when they differ.
    pub fn assert_rows(&self, expected: &[&str]) {
        let actual = self.rows();
        assert!(
            actual
                .iter()
                .map(String::as_str)
                .eq(expected.iter().copied()),
            "frame differs\n  expected: {expected:#?}\n  actual:   {actual:#?}\n{}",
            self.history()
        );
    }

    /// Asserts the focused index.
    ///
    /// # Panics
    ///
    /// Panics with the event log when the focus differs.
    pub fn assert_focus(&self, expected: Option<usize>) {
        let actual = self.component.focus();
        assert!(
            actual == expected,
            "focus is {actual:?}, expected {expected:?}\n{}",
            self.history()
        );
    }

    /// Asserts the component's value text.
    ///
    /// # Panics
    ///
    /// Panics with the event log when the value differs.
    pub fn assert_value(&self, expected: &str) {
        let actual = self.component.value();
        assert!(
            actual.as_deref() == Some(expected),
            "value is {actual:?}, expected {expected:?}\n{}",
            self.history()
        );
    }

    fn redraw(&mut self) {
        self.grid = Grid::new(self.width, self.height);
        self.clusters = ClusterTable::new();
        let area = Rect::new(0, 0, self.width, self.height);
        self.component
            .render(&mut self.grid, area, &mut self.clusters);
    }

    fn history(&self) -> String {
        let mut out = format!("virtual time {:?}; events delivered:\n", self.clock.now());
        for event in &self.log {
            let _ = writeln!(out, "  {event:?}");
        }
        out
    }
}

/// Parses a key name into a key press.
///
/// A name is a key with optional modifier prefixes, joined by `-`: `Enter`,
/// `Esc`, `Tab`, `Backspace`, `Space`, `Up`, `Down`, `Left`, `Right`, `Home`,
/// `End`, `PageUp`, `PageDown`, `Insert`, `Delete`, `F1` to `F12`, or one
/// character. Prefixes are `C-` (control), `A-` (alt) and `S-` (shift), for
/// example `C-a`, `A-Left` or `S-Tab`. Names are case-sensitive for single
/// characters and case-insensitive for the rest.
///
/// # Errors
///
/// Returns the reason when `spec` is not a key name.
pub fn parse_key(spec: &str) -> Result<KeyEvent, String> {
    let mut modifiers = Modifiers::new();
    let mut rest = spec;
    while let Some((prefix, tail)) = rest.split_once('-') {
        if tail.is_empty() {
            break;
        }
        modifiers = match prefix {
            "C" => modifiers.control(),
            "A" => modifiers.alt(),
            "S" => modifiers.shift(),
            _ => return Err(format!("unknown modifier {prefix:?} in {spec:?}")),
        };
        rest = tail;
    }
    let code = key_code(rest).ok_or_else(|| format!("unknown key {rest:?} in {spec:?}"))?;
    Ok(KeyEvent::press(code).modifiers(modifiers))
}

fn key_code(name: &str) -> Option<KeyCode> {
    let mut chars = name.chars();
    if let (Some(ch), None) = (chars.next(), chars.next()) {
        return Some(KeyCode::Char(ch));
    }
    let code = match name.to_ascii_lowercase().as_str() {
        "enter" | "return" => KeyCode::Enter,
        "esc" | "escape" => KeyCode::Esc,
        "tab" => KeyCode::Tab,
        "backspace" => KeyCode::Backspace,
        "space" => KeyCode::Char(' '),
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "insert" => KeyCode::Insert,
        "delete" => KeyCode::Delete,
        other => {
            let number = other.strip_prefix('f')?.parse::<u8>().ok()?;
            if !(1..=12).contains(&number) {
                return None;
            }
            KeyCode::F(number)
        }
    };
    Some(code)
}
