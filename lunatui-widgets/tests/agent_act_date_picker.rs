// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]
#![cfg(feature = "date-picker")]

use std::time::Duration;

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_runtime::{
    AccessibilityRole as Role, ActWith, Action, Agent, Change, HitId, HitMap, Outcome, Selector,
};
use lunatui_term::Event;
use lunatui_test::{Component, Harness};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Date, DatePicker, NamedControl, RenderCx, Widget, WidgetState};

const DUE: HitId = HitId(1);

fn day(year: i32, month: u8, day: u8) -> Result<Date, &'static str> {
    Date::from_ymd(year, month, day).ok_or("not a date")
}

struct App {
    cursor: Date,
    selected: Option<Date>,
    min: Date,
    max: Date,
    changes: Vec<Change<Outcome>>,
    agent: Agent,
    hits: HitMap,
}

impl App {
    fn picker(&self) -> DatePicker {
        let picker = DatePicker::new(self.cursor).min(self.min).max(self.max);
        match self.selected {
            Some(date) => picker.selected(date),
            None => picker,
        }
    }
}

impl Component for App {
    fn update(&mut self, _: &Event) {}

    fn tick(&mut self, _: Duration) {
        while let Some(request) = self.agent.take() {
            let context = request.context();
            let picker = self.picker();
            let change = request.apply(
                &mut ActWith(|action: &Action| {
                    picker.act(&mut self.cursor, &mut self.selected, action)
                }),
                context,
            );
            self.changes.push(change);
        }
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        let picker = self.picker();
        self.hits = HitMap::new();
        let mut cx = RenderCx::new(grid, clusters, &theme).hit_map(&mut self.hits);
        NamedControl::new(DUE, Role::Calendar, &picker)
            .name("Due date")
            .render(area, &mut cx);
    }
}

type Test = Result<(), Box<dyn std::error::Error>>;

fn mount() -> Result<Harness<App>, &'static str> {
    Ok(Harness::mount(
        App {
            cursor: day(2024, 3, 10)?,
            selected: None,
            min: day(2024, 2, 1)?,
            max: day(2024, 4, 30)?,
            changes: Vec::new(),
            agent: Agent::new(),
            hits: HitMap::new(),
        },
        21,
        8,
    ))
}

fn set(harness: &mut Harness<App>, text: &str) -> Test {
    let app = harness.component_mut();
    app.agent.act(
        &app.hits,
        Selector::new(Role::Calendar, "Due date"),
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

fn text(harness: &Harness<App>) -> String {
    harness.rows().join("\n")
}

#[test]
fn an_agent_selects_a_date_and_the_picker_shows_that_month() -> Test {
    let mut harness = mount()?;
    assert!(text(&harness).contains("March"));
    set(&mut harness, "2024-04-02")?;
    assert_eq!(harness.component().selected, Some(day(2024, 4, 2)?));
    assert_eq!(harness.component().cursor, day(2024, 4, 2)?);
    assert_eq!(last(&harness)?, Outcome::Changed);
    assert!(text(&harness).contains("April"));
    assert!(!text(&harness).contains("March"));
    Ok(())
}

#[test]
fn selecting_the_selected_date_again_changes_nothing() -> Test {
    let mut harness = mount()?;
    set(&mut harness, "2024-03-12")?;
    let rows = harness.rows();
    set(&mut harness, " 2024-03-12 ")?;
    assert_eq!(last(&harness)?, Outcome::Handled);
    assert_eq!(harness.rows(), rows);
    Ok(())
}

#[test]
fn dates_that_do_not_exist_or_lie_outside_the_bounds_are_ignored() -> Test {
    let mut harness = mount()?;
    set(&mut harness, "2024-03-12")?;
    let rows = harness.rows();
    for wrong in [
        "2024-02-30",
        "2024-05-01",
        "2024-01-31",
        "March 12",
        "",
        "2024-3-12",
    ] {
        set(&mut harness, wrong)?;
        assert_eq!(last(&harness)?, Outcome::Ignored, "{wrong:?}");
        assert_eq!(
            harness.component().selected,
            Some(day(2024, 3, 12)?),
            "{wrong:?}"
        );
    }
    assert_eq!(harness.rows(), rows);
    Ok(())
}

#[test]
fn the_bounds_include_their_own_days() -> Test {
    let mut harness = mount()?;
    set(&mut harness, "2024-02-01")?;
    assert_eq!(last(&harness)?, Outcome::Changed);
    set(&mut harness, "2024-04-30")?;
    assert_eq!(last(&harness)?, Outcome::Changed);
    Ok(())
}

#[test]
fn a_disabled_picker_and_other_actions_are_ignored() -> Test {
    let picker = DatePicker::new(day(2024, 3, 10)?).state(WidgetState::Disabled);
    let (mut cursor, mut selected) = (day(2024, 3, 10)?, None);
    assert_eq!(
        picker.act(
            &mut cursor,
            &mut selected,
            &Action::SetValue("2024-03-12".into())
        ),
        Outcome::Ignored
    );
    let open = DatePicker::new(cursor);
    assert_eq!(
        open.act(&mut cursor, &mut selected, &Action::Activate),
        Outcome::Ignored
    );
    assert_eq!(selected, None);
    assert_eq!(cursor, day(2024, 3, 10)?);
    Ok(())
}
