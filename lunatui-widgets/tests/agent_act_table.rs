// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::time::Duration;

use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_layout::Constraint;
use lunatui_runtime::{
    AccessibilityRole as Role, ActWith, Action, Agent, Change, HitId, HitMap, Outcome, Selector,
};
use lunatui_term::Event;
use lunatui_test::{Component, Harness};
use lunatui_text::source::{RowSource, VecSource};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{NamedControl, Paragraph, RenderCx, Table, TableState, Widget};

const HEADER: u32 = 10;
const ROW: u32 = 100;
const NAMES: [&str; 3] = ["Service", "p95", "Region"];

struct App {
    source: VecSource,
    state: TableState,
    changes: Vec<Change<Outcome>>,
    agent: Agent,
    hits: HitMap,
}

fn index(id: HitId, base: u32) -> usize {
    usize::try_from(id.0 - base).unwrap_or(usize::MAX)
}

impl Component for App {
    fn update(&mut self, _: &Event) {}

    fn tick(&mut self, _: Duration) {
        while let Some(request) = self.agent.take() {
            let context = request.context();
            let id = request.id;
            let change = if id.0 >= ROW {
                request.apply(
                    &mut ActWith(|action: &Action| self.state.act(index(id, ROW), action)),
                    context,
                )
            } else {
                request.apply(
                    &mut ActWith(|action: &Action| {
                        self.state
                            .act_header(&self.source, index(id, HEADER), NAMES.len(), action)
                    }),
                    context,
                )
            };
            self.changes.push(change);
        }
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        self.hits = HitMap::new();
        let mut cx = RenderCx::new(grid, clusters, &theme).hit_map(&mut self.hits);
        for (column, name) in NAMES.iter().enumerate() {
            let id = HitId(HEADER + u32::try_from(column).unwrap_or(0));
            let x = area.x + 10 * u16::try_from(column).unwrap_or(0);
            NamedControl::new(id, Role::Button, &Paragraph::new(*name))
                .name(name)
                .render(Rect::new(x, area.y, 10, 1), &mut cx);
        }
        for shown in 0..self.source.len() {
            let name = self
                .source
                .cell(self.state.source_row(shown), 0)
                .unwrap_or_default()
                .to_owned();
            let id = HitId(ROW + u32::try_from(shown).unwrap_or(0));
            let y = area.y + 1 + u16::try_from(shown).unwrap_or(0);
            NamedControl::new(id, Role::ListItem, &Paragraph::new(name.as_str()))
                .name(&name)
                .render(Rect::new(area.x, y, 10, 1), &mut cx);
        }
        Table::new(
            self.state.view(&self.source),
            vec![Constraint::Length(10); 3],
        )
        .header(NAMES.map(|name| (name, Style::new())))
        .state(&self.state)
        .render(area, &mut cx);
    }
}

type Test = Result<(), Box<dyn std::error::Error>>;

fn mount() -> Harness<App> {
    let rows = [
        ["api", "120", "eu"],
        ["db", "45", "us"],
        ["web", "300", "ap"],
    ];
    let source = VecSource::from_rows(
        rows.iter()
            .map(|row| row.iter().map(|cell| (*cell).to_owned()).collect())
            .collect(),
    );
    let mut state = TableState::new();
    state.fit(3, 3);
    Harness::mount(
        App {
            source,
            state,
            changes: Vec::new(),
            agent: Agent::new(),
            hits: HitMap::new(),
        },
        30,
        4,
    )
}

fn act(harness: &mut Harness<App>, role: Role, name: &str, action: Action) -> Test {
    let app = harness.component_mut();
    app.agent
        .act(&app.hits, Selector::new(role, name), action)?;
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

fn order(harness: &Harness<App>) -> Vec<String> {
    harness.rows()[1..]
        .iter()
        .map(|row| row.split_whitespace().next().unwrap_or("").to_owned())
        .collect()
}

#[test]
fn an_agent_puts_the_cursor_on_a_row_by_its_name() -> Test {
    let mut harness = mount();
    assert_eq!(harness.component().state.cursor(), None);
    act(&mut harness, Role::ListItem, "db", Action::Select)?;
    assert_eq!(harness.component().state.cursor(), Some(1));
    assert_eq!(last(&harness)?, Outcome::Changed);
    act(&mut harness, Role::ListItem, "web", Action::Activate)?;
    assert_eq!(harness.component().state.cursor(), Some(2));
    assert_eq!(harness.component().state.selection(), Some(2..=2));
    Ok(())
}

#[test]
fn selecting_the_row_that_holds_the_cursor_changes_nothing() -> Test {
    let mut harness = mount();
    act(&mut harness, Role::ListItem, "db", Action::Select)?;
    let before = harness.component().state.clone();
    act(&mut harness, Role::ListItem, "db", Action::Select)?;
    assert_eq!(last(&harness)?, Outcome::Handled);
    assert_eq!(harness.component().state, before);
    Ok(())
}

#[test]
fn a_header_activate_sorts_the_rows_then_reverses_them() -> Test {
    let mut harness = mount();
    assert_eq!(order(&harness), ["api", "db", "web"]);
    act(&mut harness, Role::Button, "p95", Action::Activate)?;
    assert_eq!(last(&harness)?, Outcome::Changed);
    assert_eq!(order(&harness), ["db", "api", "web"]);
    act(&mut harness, Role::Button, "p95", Action::Activate)?;
    assert_eq!(order(&harness), ["web", "api", "db"]);
    act(&mut harness, Role::Button, "Service", Action::Activate)?;
    assert_eq!(order(&harness), ["api", "db", "web"]);
    Ok(())
}

#[test]
fn rows_are_named_in_the_order_shown_so_a_sorted_table_selects_the_right_source_row() -> Test {
    let mut harness = mount();
    act(&mut harness, Role::Button, "p95", Action::Activate)?;
    act(&mut harness, Role::ListItem, "api", Action::Select)?;
    let state = &harness.component().state;
    assert_eq!(state.cursor(), Some(1));
    assert_eq!(state.source_row(1), 0);
    Ok(())
}

#[test]
fn a_missing_row_or_column_and_other_actions_are_ignored() {
    let mut state = TableState::new();
    state.fit(3, 3);
    assert_eq!(state.act(3, &Action::Select), Outcome::Ignored);
    assert_eq!(
        state.act(1, &Action::SetValue("x".into())),
        Outcome::Ignored
    );
    assert_eq!(state.act(1, &Action::Toggle), Outcome::Ignored);
    assert_eq!(state.cursor(), None);
    let source = VecSource::from_rows(vec![vec!["a".into()], vec!["b".into()], vec!["c".into()]]);
    assert_eq!(
        state.act_header(&source, 1, 1, &Action::Activate),
        Outcome::Ignored
    );
    assert_eq!(
        state.act_header(&source, 0, 1, &Action::Select),
        Outcome::Ignored
    );
    assert_eq!(state.sorted(), None);
}
