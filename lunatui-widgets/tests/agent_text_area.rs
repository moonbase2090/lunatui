// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]
#![cfg(feature = "text-area")]

use std::time::Duration;

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_runtime::{
    AccessibilityRole as Role, Act, ActError, Action, ActionKind, Agent, HitId, HitMap, Outcome,
    Selector, Snapshot,
};
use lunatui_term::Event;
use lunatui_test::{Component, Harness};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{NamedControl, RenderCx, TextArea, Widget};

const ID: HitId = HitId(7);

struct App {
    area: TextArea,
    secret: bool,
    agent: Agent,
    hits: HitMap,
    outcomes: Vec<Outcome>,
}

impl App {
    fn new(text: &str) -> Self {
        Self {
            area: TextArea::new(text),
            secret: false,
            agent: Agent::new(),
            hits: HitMap::new(),
            outcomes: Vec::new(),
        }
    }
}

impl Component for App {
    fn update(&mut self, event: &Event) {
        self.area.handle_event(event);
    }

    fn tick(&mut self, _: Duration) {
        while let Some(request) = self.agent.take() {
            assert_eq!(request.id, ID, "{request:?}");
            self.outcomes.push(self.area.act(&request.action));
        }
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        self.hits = HitMap::new();
        let mut cx = RenderCx::new(grid, clusters, &theme).hit_map(&mut self.hits);
        NamedControl::new(ID, Role::TextInput, &self.area)
            .name("Notes")
            .secret(self.secret)
            .render(area, &mut cx);
    }
}

type Test = Result<(), Box<dyn std::error::Error>>;

fn mount(text: &str) -> Harness<App> {
    Harness::mount(App::new(text), 24, 6)
}

fn act(harness: &mut Harness<App>, action: Action) -> Result<(), ActError> {
    let app = harness.component_mut();
    app.agent
        .act(&app.hits, Selector::new(Role::TextInput, "Notes"), action)?;
    harness.advance(Duration::ZERO);
    Ok(())
}

fn snapshot(harness: &Harness<App>) -> Snapshot {
    Agent::snapshot(&harness.component().hits, None)
}

#[test]
fn an_agent_finds_a_text_area_by_name_and_reads_its_text() -> Test {
    let harness = mount("first\nsecond");
    let snapshot = snapshot(&harness);
    let found = snapshot
        .find(Selector::new(Role::TextInput, "Notes"))?
        .ok_or("the text area is not a control")?;
    assert_eq!(found.value.as_deref(), Some("first\nsecond"));
    assert_eq!(found.actions, [ActionKind::Focus, ActionKind::SetValue]);
    Ok(())
}

#[test]
fn setting_the_value_replaces_the_document_and_the_screen_shows_it() -> Test {
    let mut harness = mount("old");
    act(&mut harness, Action::SetValue("one\ntwo\nthree".into()))?;
    assert_eq!(harness.component().outcomes, [Outcome::Changed]);
    assert_eq!(harness.component().area.text(), "one\ntwo\nthree");
    let rows = harness.rows();
    assert_eq!(&rows[..3], ["one", "two", "three"]);
    let value = snapshot(&harness)
        .find(Selector::new(Role::TextInput, "Notes"))?
        .and_then(|control| control.value.clone());
    assert_eq!(value.as_deref(), Some("one\ntwo\nthree"));
    Ok(())
}

#[test]
fn windows_line_endings_in_a_value_become_newlines() -> Test {
    let mut harness = mount("");
    act(&mut harness, Action::SetValue("a\r\nb\rc".into()))?;
    assert_eq!(harness.component().area.text(), "a\nb\nc");
    Ok(())
}

#[test]
fn setting_the_text_it_already_holds_is_handled_and_keeps_the_edit_history() -> Test {
    let mut harness = mount("");
    harness.type_text("ab");
    act(&mut harness, Action::SetValue("ab".into()))?;
    assert_eq!(harness.component().outcomes, [Outcome::Handled]);
    harness.key("C-z");
    assert_ne!(
        harness.component().area.text(),
        "ab",
        "an unchanged value must not drop the edit history"
    );
    Ok(())
}

#[test]
fn a_new_value_drops_the_edit_history_so_undo_cannot_bring_old_text_back() -> Test {
    let mut harness = mount("");
    harness.type_text("typed");
    act(&mut harness, Action::SetValue("agent text".into()))?;
    harness.key("C-z");
    assert_eq!(harness.component().area.text(), "agent text");
    Ok(())
}

#[test]
fn typing_after_a_value_never_lands_inside_a_character_of_the_new_text() -> Test {
    let mut harness = mount("");
    harness.type_text("xxx");
    act(&mut harness, Action::SetValue("é€".into()))?;
    harness.type_text("!");
    assert_eq!(harness.component().area.text(), "é!€");
    let mut harness = mount("");
    harness.type_text("a long first line");
    act(&mut harness, Action::SetValue("hi".into()))?;
    harness.type_text("!");
    assert_eq!(harness.component().area.text(), "hi!");
    Ok(())
}

#[test]
fn a_text_area_takes_no_toggle_or_select_and_the_state_ignores_them() {
    let mut harness = mount("keep");
    for action in [Action::Toggle, Action::Select, Action::Activate] {
        let kind = action.kind();
        assert!(
            matches!(act(&mut harness, action), Err(ActError::Unsupported(k)) if k == kind),
            "{kind:?}"
        );
    }
    assert_eq!(harness.component().area.text(), "keep");
    let mut area = TextArea::new("keep");
    assert_eq!(area.act(&Action::Toggle), Outcome::Ignored);
    assert_eq!(area.text(), "keep");
}

#[test]
fn a_secret_text_area_keeps_its_text_out_of_the_snapshot_but_still_takes_a_value() -> Test {
    let mut harness = mount("private");
    harness.component_mut().secret = true;
    harness.advance(Duration::ZERO);
    let value = snapshot(&harness)
        .find(Selector::new(Role::TextInput, "Notes"))?
        .map(|control| control.value.clone());
    assert_eq!(value, Some(None));
    act(&mut harness, Action::SetValue("changed".into()))?;
    assert_eq!(harness.component().area.text(), "changed");
    Ok(())
}

#[test]
fn registering_the_text_area_does_not_change_what_is_drawn() {
    let mut named = mount("one\ntwo");
    let mut plain = Harness::mount(
        {
            struct Plain(TextArea);
            impl Component for Plain {
                fn update(&mut self, _: &Event) {}
                fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
                    let theme = Theme::builtin(Variant::Dark);
                    self.0
                        .render(area, &mut RenderCx::new(grid, clusters, &theme));
                }
            }
            Plain(TextArea::new("one\ntwo"))
        },
        24,
        6,
    );
    named.advance(Duration::ZERO);
    plain.advance(Duration::ZERO);
    assert_eq!(named.rows(), plain.rows());
    assert_eq!(named.styled(), plain.styled());
}
