// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::time::Duration;

use lunatui_button::{Button, WidgetState};
use lunatui_choice::{CheckState, Checkbox};
use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_field::{Field, Form, FormState};
use lunatui_runtime::{
    AccessibilityRole as Role, Act, ActError, Action, ActionKind, Agent, ControlQuery,
    ControlState, HitId, HitMap, Outcome, Selector, Source,
};
use lunatui_test::{Component, Harness};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{RenderCx, Select, TextInput, Toggle, Widget};

const PLANS: [&str; 3] = ["Free", "Team", "Enterprise"];

struct App {
    name: String,
    token: String,
    notify: bool,
    terms: bool,
    plan: Option<usize>,
    saved: u32,
    focus: FormState,
    agent: Agent,
    hits: HitMap,
    applied: Vec<Source>,
}

impl App {
    fn new() -> Self {
        Self {
            name: String::new(),
            token: "hunter2".to_owned(),
            notify: false,
            terms: false,
            plan: Some(0),
            saved: 0,
            focus: FormState::new(),
            agent: Agent::new(),
            hits: HitMap::new(),
            applied: Vec::new(),
        }
    }

    fn apply(&mut self) {
        while let Some(request) = self.agent.take() {
            self.applied.push(request.source);
            let index = usize::try_from(request.id.0 - 1).unwrap_or(usize::MAX);
            if matches!(request.action, Action::Focus) {
                self.focus.focus(index);
                continue;
            }
            let outcome = match index {
                0 => self.name.act(&request.action),
                1 => self.token.act(&request.action),
                2 => self.notify.act(&request.action),
                3 => self.terms.act(&request.action),
                4 => self.pick_plan(&request.action),
                5 => {
                    self.saved += 1;
                    Outcome::Changed
                }
                _ => Outcome::Ignored,
            };
            assert!(outcome.is_taken(), "{request:?} was not applied");
        }
    }

    fn pick_plan(&mut self, action: &Action) -> Outcome {
        let Action::SetValue(value) = action else {
            return Outcome::Ignored;
        };
        match PLANS.iter().position(|plan| plan == value) {
            Some(index) => {
                self.plan = Some(index);
                Outcome::Changed
            }
            None => Outcome::Ignored,
        }
    }

    fn form_parts(
        &self,
    ) -> (
        TextInput,
        TextInput,
        Toggle,
        Select<'static>,
        Button<'static>,
    ) {
        (
            TextInput::new(self.name.clone(), self.name.len()).show_cursor(false),
            TextInput::new(self.token.clone(), self.token.len()).show_cursor(false),
            Toggle::new(self.notify),
            Select::new(&PLANS, self.plan),
            Button::new("Save"),
        )
    }
}

impl Component for App {
    fn update(&mut self, _: &lunatui_term::Event) {}

    fn tick(&mut self, _: Duration) {
        self.apply();
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        let (name, token, notify, plan, save) = self.form_parts();
        let terms = Checkbox::new("Accept terms").checked(if self.terms {
            CheckState::Checked
        } else {
            CheckState::Unchecked
        });
        let locked = TextInput::new("fixed", 0).show_cursor(false);
        let form = Form::new([
            Field::new("Name").body(&name),
            Field::new("Token").body(&token).secret(true),
            Field::new("Notifications").body(&notify),
            Field::new("Terms").name("Accept terms").body(&terms),
            Field::new("Plan").body(&plan),
            Field::new("Save").body(&save),
            Field::new("Locked").body(&locked).disabled(true),
            Field::new("Email").body(&name).error("Required"),
        ])
        .first_id(HitId(1))
        .focus(self.focus.focused());
        self.focus.sync(&form);
        self.hits = HitMap::new();
        let mut cx = RenderCx::new(grid, clusters, &theme).hit_map(&mut self.hits);
        form.render(area, &mut cx);
    }

    fn focus(&self) -> Option<usize> {
        self.focus.focused()
    }
}

fn mount() -> Harness<App> {
    Harness::mount(App::new(), 70, 30)
}

type Test = Result<(), Box<dyn std::error::Error>>;

fn act(harness: &mut Harness<App>, role: Role, name: &str, action: Action) -> Test {
    let app = harness.component_mut();
    app.agent
        .act(&app.hits, Selector::new(role, name), action)?;
    harness.advance(Duration::ZERO);
    Ok(())
}

#[test]
fn an_agent_finds_and_changes_every_control_of_a_form_by_name() -> Test {
    let mut harness = mount();
    act(
        &mut harness,
        Role::TextInput,
        "Name",
        Action::SetValue("Ada".into()),
    )?;
    act(
        &mut harness,
        Role::TextInput,
        "Token",
        Action::SetValue("s3cret".into()),
    )?;
    act(
        &mut harness,
        Role::Checkbox,
        "Notifications",
        Action::Toggle,
    )?;
    act(&mut harness, Role::Checkbox, "Accept terms", Action::Toggle)?;
    act(
        &mut harness,
        Role::ComboBox,
        "Plan",
        Action::SetValue("Team".into()),
    )?;
    act(&mut harness, Role::Button, "Save", Action::Activate)?;
    act(&mut harness, Role::TextInput, "Name", Action::Focus)?;
    let app = harness.component();
    assert_eq!(
        (
            &*app.name,
            &*app.token,
            app.notify,
            app.terms,
            app.plan,
            app.saved
        ),
        ("Ada", "s3cret", true, true, Some(1), 1)
    );
    assert_eq!(app.focus.focused(), Some(0));
    assert_eq!(app.applied, [Source::Agent; 7]);
    assert!(harness.rows().iter().any(|row| row.contains("Ada")));
    Ok(())
}

#[test]
fn the_snapshot_lists_name_role_value_state_actions_and_layer_of_every_control() -> Test {
    let mut harness = mount();
    act(
        &mut harness,
        Role::Checkbox,
        "Notifications",
        Action::Toggle,
    )?;
    act(
        &mut harness,
        Role::TextInput,
        "Name",
        Action::SetValue("Ada".into()),
    )?;
    act(&mut harness, Role::Button, "Save", Action::Focus)?;
    let app = harness.component();
    let snapshot = Agent::snapshot(
        &app.hits,
        app.focus
            .focused()
            .map(|i| HitId(1 + u32::try_from(i).unwrap_or(u32::MAX))),
    );
    let seen: Vec<_> = snapshot
        .controls()
        .iter()
        .map(|c| {
            (
                c.name.as_str(),
                c.role,
                c.value.as_deref(),
                c.checked,
                c.states.clone(),
            )
        })
        .collect();
    assert_eq!(
        seen,
        [
            ("Name", Role::TextInput, Some("Ada"), None, vec![]),
            ("Token", Role::TextInput, None, None, vec![]),
            ("Notifications", Role::Checkbox, None, Some(true), vec![]),
            ("Accept terms", Role::Checkbox, None, Some(false), vec![]),
            ("Plan", Role::ComboBox, Some("Free"), None, vec![]),
            (
                "Save",
                Role::Button,
                None,
                None,
                vec![ControlState::Focused]
            ),
            (
                "Locked",
                Role::TextInput,
                Some("fixed"),
                None,
                vec![ControlState::Disabled]
            ),
            (
                "Email",
                Role::TextInput,
                Some("Ada"),
                None,
                vec![ControlState::Invalid]
            ),
        ]
    );
    let name = snapshot.controls().first().ok_or("no controls")?;
    assert_eq!(name.actions, [ActionKind::Focus, ActionKind::SetValue]);
    assert_eq!(snapshot.controls()[6].actions, []);
    assert_eq!(name.parent, None);
    Ok(())
}

#[test]
fn an_agent_queries_real_controls_by_text_kind_and_layer() {
    let harness = mount();
    let app = harness.component();
    let names = |query| {
        Agent::query(&app.hits, None, query)
            .controls()
            .iter()
            .map(|control| control.name.clone())
            .collect::<Vec<_>>()
    };

    assert_eq!(names(ControlQuery::text("TERmS")), ["Accept terms"]);
    assert_eq!(
        names(ControlQuery::kind(Role::Checkbox)),
        ["Notifications", "Accept terms"]
    );
    assert_eq!(
        names(ControlQuery::kind(Role::TextInput).with_text("mail")),
        ["Email"]
    );
    assert_eq!(
        names(ControlQuery::layer(None)),
        [
            "Name",
            "Token",
            "Notifications",
            "Accept terms",
            "Plan",
            "Save",
            "Locked",
            "Email"
        ]
    );
    assert_eq!(
        Agent::query(&app.hits, None, ControlQuery::text("missing")).more(),
        0
    );
}

#[test]
fn a_secret_value_reaches_neither_snapshot_nor_query_json() -> Test {
    let harness = mount();
    let app = harness.component();
    let snapshot = Agent::snapshot(&app.hits, None);
    let mut json = Vec::new();
    snapshot.write_json(&mut json)?;
    let json = String::from_utf8(json)?;
    assert!(!json.contains("hunter2"));
    assert!(json.contains("\"name\":\"Token\",\"role\":\"text_input\",\"value\":null"));
    assert!(json.contains("\"name\":\"Plan\",\"role\":\"combo_box\",\"value\":\"Free\""));

    let query = Agent::query(&app.hits, None, ControlQuery::text("Token"));
    let mut query_json = Vec::new();
    query.write_json(&mut query_json)?;
    let query_json = String::from_utf8(query_json)?;
    assert!(!query_json.contains("hunter2"));
    assert!(query_json.contains("\"name\":\"Token\",\"role\":\"text_input\",\"value\":null"));
    Ok(())
}

#[test]
fn the_accessibility_export_still_carries_no_values() -> Test {
    let harness = mount();
    let mut out = Vec::new();
    harness
        .component()
        .hits
        .write_accessibility_json(&mut out, None)?;
    let out = String::from_utf8(out)?;
    assert!(!out.contains("fixed") && !out.contains("Free") && !out.contains("hunter2"));
    assert!(out.contains("\"name\":\"Plan\""));
    Ok(())
}

#[test]
fn act_refuses_disabled_unknown_unsupported_and_duplicate_controls() {
    let mut harness = mount();
    let app = harness.component_mut();
    let try_act = |agent: &mut Agent, hits: &HitMap, role, name: &str, action| {
        agent.act(hits, Selector::new(role, name), action).err()
    };
    assert_eq!(
        try_act(
            &mut app.agent,
            &app.hits,
            Role::TextInput,
            "Locked",
            Action::SetValue("x".into())
        ),
        Some(ActError::Disabled)
    );
    assert_eq!(
        try_act(
            &mut app.agent,
            &app.hits,
            Role::Button,
            "Missing",
            Action::Activate
        ),
        Some(ActError::NotFound)
    );
    assert_eq!(
        try_act(
            &mut app.agent,
            &app.hits,
            Role::Button,
            "Save",
            Action::Toggle
        ),
        Some(ActError::Unsupported(ActionKind::Toggle))
    );
    assert_eq!(
        try_act(
            &mut app.agent,
            &app.hits,
            Role::TextInput,
            "Email",
            Action::Focus
        ),
        None
    );
    assert_eq!(app.agent.pending(), 1);
}

#[test]
fn a_cancelled_request_is_never_applied() -> Test {
    let mut harness = mount();
    let app = harness.component_mut();
    let ticket = app.agent.act(
        &app.hits,
        Selector::new(Role::Button, "Save"),
        Action::Activate,
    )?;
    ticket.cancel();
    assert!(ticket.is_cancelled());
    harness.advance(Duration::ZERO);
    assert_eq!(harness.component().saved, 0);
    assert_eq!(harness.component().agent.pending(), 0);
    act(&mut harness, Role::Button, "Save", Action::Activate)?;
    assert_eq!(harness.component().saved, 1);
    Ok(())
}

#[test]
fn a_disabled_button_stops_taking_requests_once_it_is_drawn_disabled() {
    let hits = {
        let theme = Theme::builtin(Variant::Dark);
        let button = Button::new("Save").state(WidgetState::Disabled);
        let form = Form::new([Field::new("Save").body(&button)]).first_id(HitId(1));
        let mut grid = Grid::new(30, 4);
        let mut clusters = ClusterTable::new();
        let mut hits = HitMap::new();
        let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme).hit_map(&mut hits);
        form.render(Rect::new(0, 0, 30, 4), &mut cx);
        hits
    };
    let mut agent = Agent::new();
    assert_eq!(
        agent
            .act(&hits, Selector::new(Role::Button, "Save"), Action::Activate)
            .err(),
        Some(ActError::Disabled)
    );
}

#[test]
fn two_controls_with_the_same_role_and_name_are_never_guessed_between() {
    let theme = Theme::builtin(Variant::Dark);
    let one = TextInput::new("a", 0);
    let two = TextInput::new("b", 0);
    let form = Form::new([Field::new("Name").body(&one), Field::new("Name").body(&two)])
        .first_id(HitId(1));
    let mut grid = Grid::new(30, 6);
    let mut clusters = ClusterTable::new();
    let mut hits = HitMap::new();
    form.render(
        Rect::new(0, 0, 30, 6),
        &mut RenderCx::new(&mut grid, &mut clusters, &theme).hit_map(&mut hits),
    );
    let mut agent = Agent::new();
    let error = agent
        .act(&hits, Selector::new(Role::TextInput, "Name"), Action::Focus)
        .err();
    assert!(matches!(error, Some(ActError::Ambiguous(e)) if e.matches == 2));
    assert_eq!(agent.pending(), 0);
}
