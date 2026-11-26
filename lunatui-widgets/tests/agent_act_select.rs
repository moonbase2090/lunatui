// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]
#![cfg(feature = "select")]

use std::time::Duration;

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_runtime::{
    AccessibilityRole as Role, ActWith, Action, Agent, Change, HitId, HitMap, Outcome, Selector,
};
use lunatui_term::Event;
use lunatui_test::{Component, Harness};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{NamedControl, RenderCx, Select, Widget};

const THEME: HitId = HitId(1);
const OPTIONS: [&str; 3] = ["Dark", "Light", "High contrast"];

struct App {
    chosen: Option<usize>,
    changes: Vec<Change<Outcome>>,
    agent: Agent,
    hits: HitMap,
}

impl Component for App {
    fn update(&mut self, _: &Event) {}

    fn tick(&mut self, _: Duration) {
        while let Some(request) = self.agent.take() {
            let context = request.context();
            let change = request.apply(
                &mut ActWith(|action: &Action| {
                    Select::new(&OPTIONS, self.chosen).act(&mut self.chosen, action)
                }),
                context,
            );
            self.changes.push(change);
        }
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        self.hits = HitMap::new();
        let mut cx = RenderCx::new(grid, clusters, &theme).hit_map(&mut self.hits);
        NamedControl::new(
            THEME,
            Role::ComboBox,
            &Select::new(&OPTIONS, self.chosen).placeholder("pick one"),
        )
        .name("Theme")
        .render(Rect::new(area.x, area.y, area.width, 1), &mut cx);
    }
}

type Test = Result<(), Box<dyn std::error::Error>>;

fn mount() -> Harness<App> {
    Harness::mount(
        App {
            chosen: None,
            changes: Vec::new(),
            agent: Agent::new(),
            hits: HitMap::new(),
        },
        30,
        2,
    )
}

fn set(harness: &mut Harness<App>, text: &str) -> Test {
    let app = harness.component_mut();
    app.agent.act(
        &app.hits,
        Selector::new(Role::ComboBox, "Theme"),
        Action::SetValue(text.to_owned()),
    )?;
    harness.advance(Duration::ZERO);
    Ok(())
}

fn last(harness: &Harness<App>) -> Result<Outcome, &'static str> {
    harness
        .component()
        .changes
        .last()
        .and_then(|change| change.value().copied())
        .ok_or("no change was applied")
}

#[test]
fn an_agent_chooses_an_option_by_its_label_and_the_field_shows_it() -> Test {
    let mut harness = mount();
    assert!(harness.rows()[0].contains("pick one"));
    set(&mut harness, "Light")?;
    assert_eq!(harness.component().chosen, Some(1));
    assert_eq!(last(&harness)?, Outcome::Changed);
    assert!(harness.rows()[0].contains("Light"));
    set(&mut harness, "High contrast")?;
    assert_eq!(harness.component().chosen, Some(2));
    assert!(harness.rows()[0].contains("High contrast"));
    Ok(())
}

#[test]
fn choosing_the_chosen_option_again_changes_nothing() -> Test {
    let mut harness = mount();
    set(&mut harness, "Dark")?;
    let rows = harness.rows();
    set(&mut harness, "Dark")?;
    assert_eq!(last(&harness)?, Outcome::Handled);
    assert_eq!(harness.rows(), rows);
    Ok(())
}

#[test]
fn text_that_is_not_an_option_label_is_ignored_and_the_choice_stays() -> Test {
    let mut harness = mount();
    set(&mut harness, "Light")?;
    for wrong in ["light", "Lig", "Light ", "", "Solarized"] {
        set(&mut harness, wrong)?;
        assert_eq!(last(&harness)?, Outcome::Ignored, "{wrong:?}");
        assert_eq!(harness.component().chosen, Some(1), "{wrong:?}");
    }
    Ok(())
}

#[test]
fn only_set_value_takes_a_select_so_activate_is_ignored() -> Test {
    let mut harness = mount();
    let app = harness.component_mut();
    app.agent.act(
        &app.hits,
        Selector::new(Role::ComboBox, "Theme"),
        Action::Activate,
    )?;
    harness.advance(Duration::ZERO);
    assert_eq!(last(&harness)?, Outcome::Ignored);
    assert_eq!(harness.component().chosen, None);
    Ok(())
}
