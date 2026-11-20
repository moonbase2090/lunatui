// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::time::Duration;

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_field::{Field, Form};
use lunatui_runtime::{
    AccessibilityRole as Role, Act, Action, ActionKind, Agent, HitId, HitMap, Selector,
};
use lunatui_term::Event;
use lunatui_test::{Component, Harness};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{RenderCx, TextArea, Widget};

struct App {
    notes: TextArea,
    agent: Agent,
    hits: HitMap,
}

impl Component for App {
    fn update(&mut self, event: &Event) {
        self.notes.handle_event(event);
    }

    fn tick(&mut self, _: Duration) {
        while let Some(request) = self.agent.take() {
            self.notes.act(&request.action);
        }
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        let form = Form::new([Field::new("Notes").body(&self.notes)]).first_id(HitId(1));
        self.hits = HitMap::new();
        let mut cx = RenderCx::new(grid, clusters, &theme).hit_map(&mut self.hits);
        form.render(area, &mut cx);
    }
}

#[test]
fn a_text_area_in_a_field_is_a_text_input_an_agent_finds_by_the_label()
-> Result<(), Box<dyn std::error::Error>> {
    let app = App {
        notes: TextArea::new("first\nsecond"),
        agent: Agent::new(),
        hits: HitMap::new(),
    };
    let mut harness = Harness::mount(app, 40, 10);
    let selector = Selector::new(Role::TextInput, "Notes");
    let snapshot = Agent::snapshot(&harness.component().hits, None);
    let found = snapshot
        .find(selector)?
        .ok_or("the field's text area is not a control")?;
    assert_eq!(found.value.as_deref(), Some("first\nsecond"));
    assert_eq!(found.actions, [ActionKind::Focus, ActionKind::SetValue]);
    let app = harness.component_mut();
    app.agent
        .act(&app.hits, selector, Action::SetValue("replaced".into()))?;
    harness.advance(Duration::ZERO);
    assert_eq!(harness.component().notes.text(), "replaced");
    Ok(())
}
