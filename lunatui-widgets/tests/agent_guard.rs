// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]
#![cfg(feature = "text-area")]

use std::time::Duration;

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_runtime::{
    AccessibilityRole as Role, Act, ActError, Action, Agent, Change, ChangeReason, ChangeSource,
    HitId, HitMap, Outcome, Selector,
};
use lunatui_term::Event;
use lunatui_test::{Component, Harness};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{NamedControl, Paragraph, RenderCx, TextArea, Widget};

const NOTES: HitId = HitId(1);
const ALERTS: HitId = HitId(2);

struct App {
    area: TextArea,
    alerts: bool,
    refuse: bool,
    agent: Agent,
    hits: HitMap,
    changes: Vec<Change<Outcome>>,
    seen: Vec<(ChangeReason, ChangeSource)>,
}

impl App {
    fn new(refuse: bool) -> Self {
        Self {
            area: TextArea::new("draft"),
            alerts: false,
            refuse,
            agent: Agent::new(),
            hits: HitMap::new(),
            changes: Vec::new(),
            seen: Vec::new(),
        }
    }
}

impl Component for App {
    fn update(&mut self, _: &Event) {}

    fn tick(&mut self, _: Duration) {
        while let Some(request) = self.agent.take() {
            let refuse = self.refuse;
            let seen = &mut self.seen;
            let mut guard = |details: &mut lunatui_runtime::ChangeDetails| {
                seen.push((details.reason(), details.source()));
                details.cancel = refuse;
            };
            let context = request.context().guard(&mut guard);
            let change = if request.id == NOTES {
                self.area.act_with(&request.action, context)
            } else {
                self.alerts.act_with(&request.action, context)
            };
            self.changes.push(change);
        }
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        self.hits = HitMap::new();
        let mut cx = RenderCx::new(grid, clusters, &theme).hit_map(&mut self.hits);
        NamedControl::new(NOTES, Role::TextInput, &self.area)
            .name("Notes")
            .render(Rect::new(area.x, area.y, area.width, 3), &mut cx);
        let label = Paragraph::new(if self.alerts {
            "alerts on"
        } else {
            "alerts off"
        });
        NamedControl::new(ALERTS, Role::Checkbox, &label)
            .name("Alerts")
            .render(Rect::new(area.x, area.y + 4, area.width, 1), &mut cx);
    }
}

type Test = Result<(), Box<dyn std::error::Error>>;

fn act(harness: &mut Harness<App>, role: Role, name: &str, action: Action) -> Result<(), ActError> {
    let app = harness.component_mut();
    app.agent
        .act(&app.hits, Selector::new(role, name), action)?;
    harness.advance(Duration::ZERO);
    Ok(())
}

#[test]
fn a_refused_agent_edit_leaves_the_document_and_the_screen_unchanged() -> Test {
    let mut harness = Harness::mount(App::new(true), 24, 6);
    let before = harness.rows();
    act(
        &mut harness,
        Role::TextInput,
        "Notes",
        Action::SetValue("rewritten".into()),
    )?;
    let app = harness.component();
    assert_eq!(app.area.text(), "draft");
    assert_eq!(app.seen, [(ChangeReason::Edit, ChangeSource::Agent)]);
    assert!(app.changes[0].is_canceled());
    assert_eq!(app.changes[0].value(), None);
    assert_eq!(harness.rows(), before);
    Ok(())
}

#[test]
fn an_approved_agent_edit_changes_the_document_and_reports_the_edit() -> Test {
    let mut harness = Harness::mount(App::new(false), 24, 6);
    act(
        &mut harness,
        Role::TextInput,
        "Notes",
        Action::SetValue("rewritten".into()),
    )?;
    let app = harness.component();
    assert_eq!(app.area.text(), "rewritten");
    assert_eq!(app.changes[0].value(), Some(&Outcome::Changed));
    assert_eq!(app.seen, [(ChangeReason::Edit, ChangeSource::Agent)]);
    assert_eq!(&harness.rows()[0], "rewritten");
    Ok(())
}

#[test]
fn setting_the_text_it_already_holds_never_asks_the_guard() -> Test {
    let mut harness = Harness::mount(App::new(true), 24, 6);
    act(
        &mut harness,
        Role::TextInput,
        "Notes",
        Action::SetValue("draft".into()),
    )?;
    let app = harness.component();
    assert_eq!(app.seen.len(), 0);
    assert_eq!(app.changes[0].value(), None);
    assert!(!app.changes[0].is_canceled());
    Ok(())
}

#[test]
fn a_refused_toggle_keeps_the_state_and_an_approved_one_flips_it() -> Test {
    let mut refused = Harness::mount(App::new(true), 24, 6);
    act(&mut refused, Role::Checkbox, "Alerts", Action::Toggle)?;
    assert!(!refused.component().alerts);
    assert_eq!(
        refused.component().seen,
        [(ChangeReason::Toggle, ChangeSource::Agent)]
    );
    assert!(refused.component().changes[0].is_canceled());

    let mut approved = Harness::mount(App::new(false), 24, 6);
    act(&mut approved, Role::Checkbox, "Alerts", Action::Toggle)?;
    assert!(approved.component().alerts);
    assert_eq!(
        approved.component().changes[0].value(),
        Some(&Outcome::Changed)
    );
    assert!(approved.rows().iter().any(|row| row.contains("alerts on")));
    Ok(())
}

#[test]
fn an_action_the_state_does_not_take_is_not_reported_as_a_change() -> Test {
    let mut harness = Harness::mount(App::new(false), 24, 6);
    act(&mut harness, Role::TextInput, "Notes", Action::Focus)?;
    let app = harness.component();
    assert_eq!(app.changes[0].value(), None);
    assert_eq!(app.area.text(), "draft");
    Ok(())
}
