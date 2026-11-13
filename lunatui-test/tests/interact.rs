// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

//! The interaction harness delivers real, decoded events, keeps virtual time
//! and fails loudly when an assertion is wrong.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Duration;

use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_term::{
    Event, KeyCode, KeyEvent, Modifiers, MouseButton, MouseEvent, MouseKind, Scroll,
};
use lunatui_test::{Component, Harness, parse_key};

/// Draws the last typed text on row 0 and the tick time on row 1.
#[derive(Default)]
struct Probe {
    typed: String,
    ticks: Vec<Duration>,
    focus: Option<usize>,
}

impl Component for Probe {
    fn update(&mut self, event: &Event) {
        let Event::Key(key) = event else { return };
        match key.code {
            KeyCode::Char(ch) => self.typed.push(ch),
            KeyCode::Tab => self.focus = Some(self.focus.map_or(0, |n| n + 1)),
            _ => {}
        }
    }

    fn render(&mut self, grid: &mut Grid, _area: Rect, clusters: &mut ClusterTable) {
        grid.put_str(0, 0, Style::new(), &self.typed, clusters);
        let line = self
            .ticks
            .last()
            .map_or(String::new(), |t| format!("t={}ms", t.as_millis()));
        grid.put_str(0, 1, Style::new(), &line, clusters);
    }

    fn tick(&mut self, now: Duration) {
        self.ticks.push(now);
    }

    fn focus(&self) -> Option<usize> {
        self.focus
    }

    fn value(&self) -> Option<String> {
        Some(self.typed.clone())
    }
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::press(code))
}

fn mouse(kind: MouseKind, x: u16, y: u16) -> Event {
    Event::Mouse(MouseEvent::new(kind, x, y))
}

#[test]
fn key_names_parse_to_the_documented_presses() {
    let cases = [
        ("a", KeyEvent::press(KeyCode::Char('a'))),
        ("Enter", KeyEvent::press(KeyCode::Enter)),
        ("esc", KeyEvent::press(KeyCode::Esc)),
        ("Space", KeyEvent::press(KeyCode::Char(' '))),
        ("PageDown", KeyEvent::press(KeyCode::PageDown)),
        ("F5", KeyEvent::press(KeyCode::F(5))),
        (
            "C-a",
            KeyEvent::press(KeyCode::Char('a')).modifiers(Modifiers::new().control()),
        ),
        (
            "A-Left",
            KeyEvent::press(KeyCode::Left).modifiers(Modifiers::new().alt()),
        ),
        (
            "C-A-x",
            KeyEvent::press(KeyCode::Char('x')).modifiers(Modifiers::new().control().alt()),
        ),
        ("-", KeyEvent::press(KeyCode::Char('-'))),
        (
            "C--",
            KeyEvent::press(KeyCode::Char('-')).modifiers(Modifiers::new().control()),
        ),
    ];
    for (spec, expected) in cases {
        assert_eq!(parse_key(spec), Ok(expected), "{spec}");
    }
}

#[test]
fn bad_key_names_say_what_is_wrong() {
    assert_eq!(
        parse_key("Hyper"),
        Err("unknown key \"Hyper\" in \"Hyper\"".to_owned())
    );
    assert_eq!(
        parse_key("X-a"),
        Err("unknown modifier \"X\" in \"X-a\"".to_owned())
    );
    assert_eq!(
        parse_key("F13"),
        Err("unknown key \"F13\" in \"F13\"".to_owned())
    );
}

#[test]
fn keys_reach_the_component_as_the_events_a_terminal_would_decode() {
    let mut harness = Harness::mount(Probe::default(), 10, 2);
    harness.keys("Tab Esc Enter Up C-a S-Tab Backspace");
    assert_eq!(
        harness.events(),
        [
            key(KeyCode::Tab),
            key(KeyCode::Esc),
            key(KeyCode::Enter),
            key(KeyCode::Up),
            Event::Key(KeyEvent::press(KeyCode::Char('a')).modifiers(Modifiers::new().control())),
            Event::Key(KeyEvent::press(KeyCode::Tab).modifiers(Modifiers::new().shift())),
            key(KeyCode::Backspace),
        ]
    );
}

#[test]
fn typed_text_arrives_one_character_at_a_time_and_is_drawn() {
    let mut harness = Harness::mount(Probe::default(), 10, 2);
    harness.type_text("héllo");
    harness.assert_value("héllo");
    harness.assert_rows(&["héllo", ""]);
}

#[test]
fn a_click_is_a_press_then_a_release_at_the_same_cell() {
    let mut harness = Harness::mount(Probe::default(), 10, 2);
    harness.click(3, 1);
    harness.click_with(MouseButton::Right, 0, 0);
    assert_eq!(
        harness.events(),
        [
            mouse(MouseKind::Down(MouseButton::Left), 3, 1),
            mouse(MouseKind::Up(MouseButton::Left), 3, 1),
            mouse(MouseKind::Down(MouseButton::Right), 0, 0),
            mouse(MouseKind::Up(MouseButton::Right), 0, 0),
        ]
    );
}

#[test]
fn hover_drag_and_scroll_are_the_pointer_events_they_name() {
    let mut harness = Harness::mount(Probe::default(), 10, 2);
    harness
        .hover(2, 0)
        .drag((1, 0), (4, 1))
        .scroll(Scroll::Down, 5, 1);
    assert_eq!(
        harness.events(),
        [
            mouse(MouseKind::Move, 2, 0),
            mouse(MouseKind::Down(MouseButton::Left), 1, 0),
            mouse(MouseKind::Drag(MouseButton::Left), 4, 1),
            mouse(MouseKind::Up(MouseButton::Left), 4, 1),
            mouse(MouseKind::Scroll(Scroll::Down), 5, 1),
        ]
    );
}

#[test]
fn advance_moves_virtual_time_and_ticks_once_per_step() {
    let mut harness = Harness::mount(Probe::default(), 10, 2);
    harness.advance(Duration::from_millis(100));
    harness.advance(Duration::from_millis(250));
    assert_eq!(harness.now(), Duration::from_millis(350));
    assert_eq!(
        harness.component().ticks,
        [Duration::from_millis(100), Duration::from_millis(350)]
    );
    harness.assert_rows(&["", "t=350ms"]);
}

#[test]
fn focus_and_value_assertions_follow_the_component() {
    let mut harness = Harness::mount(Probe::default(), 10, 2);
    harness.assert_focus(None);
    harness.keys("Tab Tab");
    harness.assert_focus(Some(1));
}

/// The panic message of `run`, or an empty string when it did not panic.
fn panic_message(run: impl FnOnce()) -> String {
    match catch_unwind(AssertUnwindSafe(run)) {
        Ok(()) => String::new(),
        Err(error) => error.downcast_ref::<String>().cloned().unwrap_or_default(),
    }
}

#[test]
fn a_wrong_assertion_fails_and_names_the_frame_and_the_events() {
    let mut harness = Harness::mount(Probe::default(), 10, 2);
    harness.type_text("ab");
    let rows = panic_message(|| harness.assert_rows(&["xy", ""]));
    assert!(rows.contains("frame differs"), "{rows}");
    assert!(rows.contains("\"ab\""), "{rows}");
    assert!(rows.contains("Char('b')"), "{rows}");
    let focus = panic_message(|| harness.assert_focus(Some(0)));
    assert!(focus.contains("focus is None, expected Some(0)"), "{focus}");
    let value = panic_message(|| harness.assert_value("abc"));
    assert!(
        value.contains("value is Some(\"ab\"), expected \"abc\""),
        "{value}"
    );
}

#[test]
fn matching_assertions_do_not_fail() {
    let mut harness = Harness::mount(Probe::default(), 10, 2);
    harness.type_text("ab");
    assert_eq!(panic_message(|| harness.assert_rows(&["ab", ""])), "");
    assert_eq!(panic_message(|| harness.assert_value("ab")), "");
    assert_eq!(panic_message(|| harness.assert_focus(None)), "");
}

#[test]
fn an_unknown_key_name_fails_the_test() {
    let mut harness = Harness::mount(Probe::default(), 10, 2);
    let text = panic_message(|| {
        harness.key("Hyper");
    });
    assert!(text.contains("bad key \"Hyper\""), "{text}");
}

#[test]
fn resize_redraws_at_the_new_size() {
    let mut harness = Harness::mount(Probe::default(), 10, 2);
    harness.type_text("abcdef");
    harness.resize(4, 1);
    harness.assert_rows(&["abcd"]);
}

#[test]
fn the_harness_never_reads_the_wall_clock_or_sleeps() {
    let source = include_str!("../src/interact.rs");
    for banned in [
        "Instant::now",
        "SystemTime",
        "sleep(",
        "thread::",
        "timeout",
    ] {
        assert!(!source.contains(banned), "interact.rs uses {banned}");
    }
}
