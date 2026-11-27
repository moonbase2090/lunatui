// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]
#![cfg(feature = "radio")]

use std::time::Duration;

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_runtime::{
    AccessibilityRole as Role, ActWith, Action, Agent, Change, HitId, HitMap, Outcome, Selector,
};
use lunatui_term::Event;
use lunatui_test::{Component, Harness};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{NamedControl, Paragraph, RadioGroup, RenderCx, Widget};

const FIRST: u32 = 1;
const LABELS: [&str; 3] = ["Daily", "Weekly", "Monthly"];

struct App {
    chosen: Option<usize>,
    changes: Vec<Change<Outcome>>,
    agent: Agent,
    hits: HitMap,
}

fn group(chosen: Option<usize>) -> RadioGroup {
    RadioGroup::new(LABELS).select(chosen).disabled_options([2])
}

impl Component for App {
    fn update(&mut self, _: &Event) {}

    fn tick(&mut self, _: Duration) {
        while let Some(request) = self.agent.take() {
            let context = request.context();
            let index = usize::try_from(request.id.0 - FIRST).unwrap_or(usize::MAX);
            let change = request.apply(
                &mut ActWith(|action: &Action| {
                    group(self.chosen).act(&mut self.chosen, index, action)
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
        let radio = group(self.chosen);
        for (index, rect) in radio.option_areas(area) {
            let id = HitId(FIRST + u32::try_from(index).unwrap_or(0));
            NamedControl::new(id, Role::RadioButton, &Paragraph::new(LABELS[index]))
                .name(LABELS[index])
                .disabled(!radio.is_enabled(index))
                .render(rect, &mut cx);
        }
        radio.render(area, &mut cx);
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
        20,
        3,
    )
}

fn select(harness: &mut Harness<App>, name: &str) -> Test {
    let app = harness.component_mut();
    app.agent.act(
        &app.hits,
        Selector::new(Role::RadioButton, name),
        Action::Select,
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
fn an_agent_chooses_an_option_and_the_filled_glyph_moves() -> Test {
    let mut harness = mount();
    assert!(harness.rows().iter().all(|row| !row.contains('◉')));
    select(&mut harness, "Weekly")?;
    assert_eq!(harness.component().chosen, Some(1));
    assert_eq!(last(&harness)?, Outcome::Changed);
    assert!(harness.rows()[1].contains('◉'));
    select(&mut harness, "Daily")?;
    assert_eq!(harness.component().chosen, Some(0));
    assert!(harness.rows()[0].contains('◉'));
    assert!(!harness.rows()[1].contains('◉'));
    Ok(())
}

#[test]
fn choosing_the_chosen_option_again_changes_nothing() -> Test {
    let mut harness = mount();
    select(&mut harness, "Weekly")?;
    let rows = harness.rows();
    select(&mut harness, "Weekly")?;
    assert_eq!(last(&harness)?, Outcome::Handled);
    assert_eq!(harness.rows(), rows);
    Ok(())
}

#[test]
fn a_disabled_option_cannot_be_chosen_by_an_agent() -> Test {
    let mut harness = mount();
    select(&mut harness, "Daily")?;
    let app = harness.component_mut();
    assert!(
        app.agent
            .act(
                &app.hits,
                Selector::new(Role::RadioButton, "Monthly"),
                Action::Select
            )
            .is_err()
    );
    assert_eq!(
        group(None).act(&mut None, 2, &Action::Select),
        Outcome::Ignored
    );
    assert_eq!(harness.component().chosen, Some(0));
    Ok(())
}

#[test]
fn a_missing_option_and_other_actions_are_ignored() {
    let radio = group(None);
    let mut chosen = None;
    assert_eq!(radio.act(&mut chosen, 3, &Action::Select), Outcome::Ignored);
    assert_eq!(
        radio.act(&mut chosen, 0, &Action::Activate),
        Outcome::Ignored
    );
    assert_eq!(radio.act(&mut chosen, 0, &Action::Toggle), Outcome::Ignored);
    assert_eq!(chosen, None);
}
