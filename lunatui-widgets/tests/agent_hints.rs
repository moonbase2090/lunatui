// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![cfg(all(feature = "nested-menu", feature = "key-hints"))]
#![doc = include_str!("../README.md")]

use std::time::Duration;

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_runtime::keymap::{Command, KeyChord, Keymap};
use lunatui_runtime::{
    AccessibilityRole as Role, Action, Agent, ChangeContext, ChangeSource, Handle, HitId, HitMap,
    Selector, Snapshot,
};
use lunatui_term::{Event, KeyCode, Modifiers};
use lunatui_test::{Component, Harness};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{
    ItemControls, KeyHintBar, Menu, MenuEvent, MenuItem, MenuState, RenderCx, Widget,
};

const SAVE: &[KeyChord] = &[KeyChord::new(
    KeyCode::Char('s'),
    Modifiers::new().control(),
)];
const QUIT: &[KeyChord] = &[KeyChord::new(KeyCode::Char('q'), Modifiers::new())];
const FIRST: u32 = 300;

fn keymap() -> Keymap {
    let mut keymap = Keymap::new();
    keymap.register(Command::new("save", "Save", SAVE));
    keymap.register(Command::new("unbound", "Unbound", &[]));
    keymap.register(Command::new("quit", "Quit", QUIT));
    keymap
}

struct Hints {
    keymap: Keymap,
    width: u16,
    controls: bool,
    agent: Agent,
    hits: HitMap,
    ran: Vec<&'static str>,
}

impl Hints {
    fn new(width: u16, controls: bool) -> Self {
        Self {
            keymap: keymap(),
            width,
            controls,
            agent: Agent::new(),
            hits: HitMap::new(),
            ran: Vec::new(),
        }
    }
}

impl Component for Hints {
    fn update(&mut self, _: &Event) {}

    fn tick(&mut self, _: Duration) {
        while let Some(request) = self.agent.take() {
            let index = usize::try_from(request.id.0 - FIRST).unwrap_or(usize::MAX);
            let hinted = self
                .keymap
                .commands()
                .iter()
                .filter(|command| !command.chords().is_empty())
                .nth(index);
            if let (Action::Activate, Some(command)) = (request.action, hinted) {
                self.ran.push(command.id());
            }
        }
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        self.hits = HitMap::new();
        let mut cx = RenderCx::new(grid, clusters, &theme).hit_map(&mut self.hits);
        let bar = KeyHintBar::new(&self.keymap);
        let bar = if self.controls {
            bar.controls(ItemControls::new(HitId(FIRST)))
        } else {
            bar
        };
        bar.render(
            Rect::new(area.x, area.y, self.width.min(area.width), 1),
            &mut cx,
        );
    }
}

type Test = Result<(), Box<dyn std::error::Error>>;

fn snapshot<C: Component>(harness: &Harness<C>, hits: impl Fn(&C) -> &HitMap) -> Snapshot {
    Agent::snapshot(hits(harness.component()), None)
}

#[test]
fn every_key_hint_drawn_in_full_is_a_button_with_the_chord_the_row_shows() {
    let harness = Harness::mount(Hints::new(40, true), 40, 1);
    let snapshot = snapshot(&harness, |app| &app.hits);
    let seen: Vec<_> = snapshot
        .controls()
        .iter()
        .map(|c| (c.name.as_str(), c.role, c.shortcut.as_deref(), c.id.0))
        .collect();
    assert_eq!(
        seen,
        [
            ("Save", Role::Button, Some("Ctrl+S"), 300),
            ("Quit", Role::Button, Some("Q"), 301)
        ],
        "a command with no chord is not a hint and takes no id"
    );
    let row = harness.rows().join("");
    for control in snapshot.controls() {
        let at = usize::from(control.area.x);
        let width = usize::from(control.area.width);
        let shown: String = row.chars().skip(at).take(width).collect();
        let shortcut = control.shortcut.as_deref().unwrap_or("");
        assert_eq!(shown, format!("[{shortcut}] {}", control.name));
    }
}

#[test]
fn a_hint_clipped_by_the_row_is_not_a_control() {
    let harness = Harness::mount(Hints::new(20, true), 40, 1);
    let names: Vec<_> = snapshot(&harness, |app| &app.hits)
        .controls()
        .iter()
        .map(|c| c.name.clone())
        .collect();
    assert_eq!(names, ["Save"], "'[Q] Quit' does not fit in 20 columns");
    let row = harness.rows().join("");
    assert!(row.contains("[Q] Q") && !row.contains("Quit"), "{row:?}");
}

#[test]
fn an_agent_activates_a_hint_and_the_app_runs_that_command() -> Test {
    let mut harness = Harness::mount(Hints::new(40, true), 40, 1);
    let app = harness.component_mut();
    app.agent.act(
        &app.hits,
        Selector::new(Role::Button, "Quit"),
        Action::Activate,
    )?;
    harness.advance(Duration::ZERO);
    assert_eq!(harness.component().ran, ["quit"]);
    Ok(())
}

#[test]
fn registering_the_hints_does_not_change_what_is_drawn() {
    let plain = Harness::mount(Hints::new(40, false), 40, 1);
    let named = Harness::mount(Hints::new(40, true), 40, 1);
    assert_eq!(plain.rows(), named.rows());
    assert_eq!(plain.styled(), named.styled());
    assert_eq!(snapshot(&plain, |app| &app.hits).controls(), []);
}

#[test]
fn the_snapshot_json_carries_the_shortcut_and_the_mnemonic() -> Test {
    let harness = Harness::mount(Hints::new(40, true), 40, 1);
    let mut json = Vec::new();
    snapshot(&harness, |app| &app.hits).write_json(&mut json)?;
    let json = String::from_utf8(json)?;
    assert!(json.contains("\"name\":\"Save\""));
    assert!(json.contains("\"shortcut\":\"Ctrl+S\",\"mnemonic\":null"));
    assert!(json.contains("\"shortcut\":\"Q\""));
    Ok(())
}

fn items() -> Vec<MenuItem<&'static str>> {
    vec![
        MenuItem::group(
            "File",
            vec![
                MenuItem::item("New", "new").mnemonic('n'),
                MenuItem::item("Open", "open").mnemonic('o'),
                MenuItem::item("Plain", "plain"),
                MenuItem::item("Print", "print")
                    .mnemonic('p')
                    .disabled(true),
            ],
        )
        .mnemonic('f'),
        MenuItem::item("Help", "help").mnemonic('h'),
    ]
}

struct Bar {
    state: MenuState<&'static str>,
    agent: Agent,
    hits: HitMap,
    chosen: Vec<&'static str>,
}

impl Bar {
    fn new() -> Self {
        Self {
            state: MenuState::new(items()),
            agent: Agent::new(),
            hits: HitMap::new(),
            chosen: Vec::new(),
        }
    }

    fn collect(&mut self) {
        for event in self.state.drain_events() {
            if let MenuEvent::Selected { value, .. } = event {
                self.chosen.push(value);
            }
        }
    }
}

impl Component for Bar {
    fn update(&mut self, event: &Event) {
        self.state
            .handle_with(event, ChangeContext::for_event(event));
        self.collect();
    }

    fn tick(&mut self, _: Duration) {
        while let Some(request) = self.agent.take() {
            let index = usize::try_from(request.id.0 - FIRST).unwrap_or(usize::MAX);
            let context = ChangeContext::new(ChangeSource::Agent);
            match request.action {
                Action::Select => {
                    self.state.highlight_with(index, context);
                }
                Action::Activate => {
                    self.state.activate_with(index, context);
                }
                _ => {}
            }
        }
        self.collect();
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        self.hits = HitMap::new();
        let mut cx = RenderCx::new(grid, clusters, &theme).hit_map(&mut self.hits);
        Menu::new(&self.state)
            .controls(ItemControls::new(HitId(FIRST)))
            .render(area, &mut cx);
    }
}

fn open_file(harness: &mut Harness<Bar>) -> Test {
    let app = harness.component_mut();
    app.agent.act(
        &app.hits,
        Selector::new(Role::MenuItem, "File"),
        Action::Select,
    )?;
    harness.advance(Duration::ZERO);
    Ok(())
}

#[test]
fn menu_items_report_the_mnemonic_the_menu_underlines() -> Test {
    let mut harness = Harness::mount(Bar::new(), 40, 10);
    open_file(&mut harness)?;
    let snapshot = snapshot(&harness, |app| &app.hits);
    let seen: Vec<_> = snapshot
        .controls()
        .iter()
        .map(|c| (c.name.as_str(), c.mnemonic))
        .collect();
    assert_eq!(
        seen,
        [
            ("File", Some('f')),
            ("Help", Some('h')),
            ("New", Some('n')),
            ("Open", Some('o')),
            ("Plain", None),
            ("Print", Some('p')),
        ]
    );
    Ok(())
}

#[test]
fn the_mnemonic_key_and_the_agent_choose_the_same_item() -> Test {
    let mut by_key = Harness::mount(Bar::new(), 40, 10);
    open_file(&mut by_key)?;
    by_key.key("o");

    let mut by_agent = Harness::mount(Bar::new(), 40, 10);
    open_file(&mut by_agent)?;
    let app = by_agent.component_mut();
    app.agent.act(
        &app.hits,
        Selector::new(Role::MenuItem, "Open"),
        Action::Activate,
    )?;
    by_agent.advance(Duration::ZERO);

    assert_eq!(by_key.component().chosen, ["open"]);
    assert_eq!(by_agent.component().chosen, ["open"]);
    assert_eq!(by_key.rows(), by_agent.rows());
    Ok(())
}
