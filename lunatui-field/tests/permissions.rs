// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::time::Duration;

use lunatui_button::{Button, Variant as ButtonVariant};
use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_field::Field;
use lunatui_runtime::{
    AccessibilityRole as Role, ActError, Action, Agent, ConfirmId, HitId, HitMap, Permission,
    Selector,
};
use lunatui_term::{Event, KeyCode};
use lunatui_test::{Component, Harness};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{RenderCx, Widget};

const SAVE: HitId = HitId(1);
const DELETE: HitId = HitId(2);
const PROMPT_ROW: u16 = 7;

struct App {
    draft: Option<&'static str>,
    saves: u32,
    show_delete: bool,
    delete_disabled: bool,
    delete_id: HitId,
    agent: Agent,
    hits: HitMap,
    confirmed: Option<Result<(), ActError>>,
}

impl App {
    fn new() -> Self {
        Self {
            draft: Some("notes"),
            saves: 0,
            show_delete: true,
            delete_disabled: false,
            delete_id: DELETE,
            agent: Agent::new(),
            hits: HitMap::new(),
            confirmed: None,
        }
    }
}

impl Component for App {
    fn update(&mut self, event: &Event) {
        let Event::Key(key) = event else {
            return;
        };
        let Some(waiting) = self.agent.awaiting().first().map(|waiting| waiting.id) else {
            return;
        };
        match key.code {
            KeyCode::Char('y') => {
                self.confirmed = Some(self.agent.confirm(&self.hits, waiting).map(|_| ()));
            }
            KeyCode::Char('n') => {
                self.agent.decline(waiting);
            }
            _ => {}
        }
    }

    fn tick(&mut self, _: Duration) {
        while let Some(request) = self.agent.take() {
            match (request.id, &request.action) {
                (SAVE, Action::Activate) => self.saves += 1,
                (DELETE, Action::Activate) => self.draft = None,
                _ => {}
            }
        }
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        self.hits = HitMap::new();
        let mut cx = RenderCx::new(grid, clusters, &theme).hit_map(&mut self.hits);
        let save = Button::new("Save");
        Field::new("Save")
            .body(&save)
            .id(SAVE)
            .render(Rect::new(0, 0, area.width, 3), &mut cx);
        if self.show_delete {
            let delete = Button::new("Delete").variant(ButtonVariant::Destructive);
            Field::new("Delete draft")
                .body(&delete)
                .id(self.delete_id)
                .destructive(true)
                .disabled(self.delete_disabled)
                .render(Rect::new(0, 3, area.width, 3), &mut cx);
        }
        if let Some(waiting) = self.agent.awaiting().first() {
            let prompt = format!("Agent: {:?} {}? y/n", waiting.action, waiting.name);
            cx.put_str(0, PROMPT_ROW, &prompt, Style::default());
        }
    }
}

type Test = Result<(), Box<dyn std::error::Error>>;

fn send_and_sync<T: Send + Sync>(_: &T) {}

fn mount() -> Harness<App> {
    Harness::mount(App::new(), 48, 10)
}

fn act(harness: &mut Harness<App>, name: &str, action: Action) -> Result<(), ActError> {
    let app = harness.component_mut();
    app.agent
        .act(&app.hits, Selector::new(Role::Button, name), action)
        .map(|_| ())
}

fn waiting_id(result: Result<(), ActError>) -> Result<ConfirmId, Box<dyn std::error::Error>> {
    match result {
        Err(ActError::NeedsConfirmation(id)) => Ok(id),
        other => Err(format!("expected a request waiting for confirmation, got {other:?}").into()),
    }
}

#[test]
fn a_destructive_action_waits_for_the_person_and_a_plain_one_does_not() -> Test {
    let mut harness = mount();
    act(&mut harness, "Save", Action::Activate)?;
    harness.advance(Duration::ZERO);
    assert_eq!(harness.component().saves, 1, "Save is allowed by default");

    let id = waiting_id(act(&mut harness, "Delete draft", Action::Activate))?;
    harness.advance(Duration::ZERO);
    let app = harness.component();
    assert_eq!(
        app.draft,
        Some("notes"),
        "nothing is deleted before the person agrees"
    );
    assert_eq!(app.agent.pending(), 0, "the waiting request is not queued");
    let [waiting] = app.agent.awaiting() else {
        return Err(format!("one waiting request, got {:?}", app.agent.awaiting()).into());
    };
    assert_eq!(
        (waiting.id, waiting.control, waiting.name.as_str()),
        (id, DELETE, "Delete draft")
    );
    assert_eq!(
        (waiting.role, &waiting.action),
        (Role::Button, &Action::Activate)
    );
    assert_eq!(
        harness.rows()[usize::from(PROMPT_ROW)],
        "Agent: Activate Delete draft? y/n"
    );

    harness.key("y");
    harness.advance(Duration::ZERO);
    let app = harness.component();
    assert_eq!(app.confirmed, Some(Ok(())));
    assert_eq!(app.draft, None, "the confirmed request was applied");
    assert_eq!(app.agent.awaiting(), []);
    assert_eq!(harness.rows()[usize::from(PROMPT_ROW)], "");
    Ok(())
}

#[test]
fn the_snapshot_marks_the_destructive_control() -> Test {
    let harness = mount();
    let snapshot = Agent::snapshot(&harness.component().hits, None);
    let marks: Vec<(&str, bool)> = snapshot
        .controls()
        .iter()
        .map(|control| (control.name.as_str(), control.destructive))
        .collect();
    assert_eq!(marks, [("Save", false), ("Delete draft", true)]);
    let mut json = Vec::new();
    snapshot.write_json(&mut json)?;
    let json = String::from_utf8(json)?;
    assert!(json.contains("\"name\":\"Save\",\"role\":\"button\",\"value\":null,\"checked\":null,\"shortcut\":null,\"mnemonic\":null,\"destructive\":false"));
    assert!(json.contains("\"name\":\"Delete draft\",\"role\":\"button\",\"value\":null,\"checked\":null,\"shortcut\":null,\"mnemonic\":null,\"destructive\":true"));
    Ok(())
}

#[test]
fn focusing_a_destructive_control_needs_no_confirmation() -> Test {
    let mut harness = mount();
    act(&mut harness, "Delete draft", Action::Focus)?;
    let app = harness.component();
    assert_eq!(app.agent.pending(), 1);
    assert_eq!(app.agent.awaiting(), []);
    Ok(())
}

#[test]
fn a_declined_request_is_dropped_and_asking_again_waits_again() -> Test {
    let mut harness = mount();
    let first = waiting_id(act(&mut harness, "Delete draft", Action::Activate))?;
    harness.key("n");
    harness.advance(Duration::ZERO);
    let app = harness.component_mut();
    assert_eq!(app.agent.awaiting(), [], "declining drops it");
    assert_eq!(app.draft, Some("notes"));
    assert_eq!(app.confirmed, None);
    assert_eq!(
        app.agent.confirm(&app.hits, first).map(|_| ()),
        Err(ActError::NotFound),
        "a declined request cannot be confirmed later"
    );
    assert!(!app.agent.decline(first), "nor declined twice");

    let second = waiting_id(act(&mut harness, "Delete draft", Action::Activate))?;
    assert_ne!(
        first, second,
        "the agent cannot get past the prompt by asking again"
    );
    harness.advance(Duration::ZERO);
    assert_eq!(harness.component().draft, Some("notes"));
    Ok(())
}

#[test]
fn confirming_after_the_control_went_away_was_replaced_or_was_disabled_applies_nothing() -> Test {
    let mut harness = mount();
    waiting_id(act(&mut harness, "Delete draft", Action::Activate))?;
    harness.component_mut().show_delete = false;
    harness.advance(Duration::ZERO);
    harness.key("y");
    harness.advance(Duration::ZERO);
    let app = harness.component();
    assert_eq!(app.confirmed, Some(Err(ActError::NotFound)));
    assert!(
        app.agent.awaiting().is_empty(),
        "a stale request is dropped"
    );
    assert_eq!(app.draft, Some("notes"));

    let mut harness = mount();
    waiting_id(act(&mut harness, "Delete draft", Action::Activate))?;
    harness.component_mut().delete_id = HitId(3);
    harness.advance(Duration::ZERO);
    harness.key("y");
    harness.advance(Duration::ZERO);
    let app = harness.component();
    assert_eq!(
        app.confirmed,
        Some(Err(ActError::NotFound)),
        "another control under the same name is not the one the agent asked about"
    );
    assert_eq!(app.agent.pending(), 0);
    assert_eq!(app.draft, Some("notes"));

    let mut harness = mount();
    waiting_id(act(&mut harness, "Delete draft", Action::Activate))?;
    harness.component_mut().delete_disabled = true;
    harness.advance(Duration::ZERO);
    harness.key("y");
    harness.advance(Duration::ZERO);
    let app = harness.component();
    assert_eq!(app.confirmed, Some(Err(ActError::Disabled)));
    assert_eq!(app.agent.awaiting(), []);
    assert_eq!(app.draft, Some("notes"));
    Ok(())
}

#[test]
fn an_allow_hook_can_deny_and_fall_back_on_the_default() -> Test {
    let mut harness = mount();
    harness.component_mut().agent.allow(|control, action| {
        if control.name == "Save" {
            Permission::Deny
        } else {
            Permission::default_for(control, action)
        }
    });
    send_and_sync(&harness.component().agent);
    assert_eq!(
        act(&mut harness, "Save", Action::Activate),
        Err(ActError::Denied)
    );
    assert_eq!(harness.component().agent.pending(), 0);
    harness.advance(Duration::ZERO);
    assert_eq!(
        harness.component().saves,
        0,
        "a denied action never reaches the app"
    );
    waiting_id(act(&mut harness, "Delete draft", Action::Activate))?;
    Ok(())
}

#[test]
fn an_allow_hook_can_let_a_destructive_action_through() -> Test {
    let mut harness = mount();
    harness.component_mut().agent.allow(|control, _| {
        assert!(control.is_destructive() || control.name == "Save");
        Permission::Allow
    });
    act(&mut harness, "Delete draft", Action::Activate)?;
    harness.advance(Duration::ZERO);
    let app = harness.component();
    assert_eq!(app.agent.awaiting(), []);
    assert_eq!(app.draft, None);
    Ok(())
}
