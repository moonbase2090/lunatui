// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::time::Duration;

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_runtime::{
    AccessibilityRole as Role, ActError, Action, Agent, ControlState, HitId, HitMap, Selector,
    Snapshot,
};
use lunatui_test::{Component, Harness};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{ItemControls, List, RenderCx, Tabs, VecSource, Widget};

const TABS: [&str; 3] = ["Inbox", "Sent", "Drafts"];
const ROWS: [&str; 6] = ["Ada", "Grace", "Ada", "", "Linus", "Margaret"];
const TAB_FIRST: u32 = 100;
const ROW_FIRST: u32 = 200;

struct App {
    tab: usize,
    row: usize,
    offset: usize,
    controls: bool,
    agent: Agent,
    hits: HitMap,
}

impl App {
    fn new(controls: bool) -> Self {
        Self {
            tab: 0,
            row: 0,
            offset: 0,
            controls,
            agent: Agent::new(),
            hits: HitMap::new(),
        }
    }

    fn tabs(&self) -> Tabs {
        let tabs = Tabs::new(TABS).select(self.tab);
        if self.controls {
            tabs.controls(ItemControls::new(HitId(TAB_FIRST)))
        } else {
            tabs
        }
    }

    fn list(&self) -> List<VecSource> {
        let rows = ROWS.iter().map(|row| vec![(*row).to_owned()]).collect();
        let list = List::new(VecSource::from_rows(rows))
            .selected(Some(self.row))
            .offset(self.offset);
        if self.controls {
            list.controls(ItemControls::new(HitId(ROW_FIRST)))
        } else {
            list
        }
    }
}

impl Component for App {
    fn update(&mut self, _: &lunatui_term::Event) {}

    fn tick(&mut self, _: Duration) {
        while let Some(request) = self.agent.take() {
            if let Action::Select = request.action {
                match request.id.0 {
                    id @ 100..200 => self.tab = usize::try_from(id - TAB_FIRST).unwrap_or(0),
                    id => self.row = usize::try_from(id - ROW_FIRST).unwrap_or(0),
                }
            }
        }
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        let (tabs, list) = (self.tabs(), self.list());
        self.hits = HitMap::new();
        let mut cx = RenderCx::new(grid, clusters, &theme).hit_map(&mut self.hits);
        tabs.render(Rect::new(0, 0, area.width, 1), &mut cx);
        list.render(Rect::new(0, 2, area.width, 6), &mut cx);
    }
}

type Test = Result<(), Box<dyn std::error::Error>>;

fn select(harness: &mut Harness<App>, role: Role, name: &str) -> Result<(), ActError> {
    let app = harness.component_mut();
    app.agent
        .act(&app.hits, Selector::new(role, name), Action::Select)?;
    harness.advance(Duration::ZERO);
    Ok(())
}

fn snapshot(harness: &Harness<App>) -> Snapshot {
    Agent::snapshot(&harness.component().hits, None)
}

#[test]
fn an_agent_selects_a_tab_and_a_list_row_by_name() -> Test {
    let mut harness = Harness::mount(App::new(true), 30, 8);
    select(&mut harness, Role::Tab, "Drafts")?;
    select(&mut harness, Role::ListItem, "Linus")?;
    let app = harness.component();
    assert_eq!((app.tab, app.row), (2, 4));
    let snapshot = snapshot(&harness);
    let chosen = |role, name| {
        snapshot
            .find(Selector::new(role, name))
            .ok()
            .flatten()
            .is_some_and(|control| control.states.contains(&ControlState::Selected))
    };
    assert!(chosen(Role::Tab, "Drafts"));
    assert!(!chosen(Role::Tab, "Inbox"));
    assert!(chosen(Role::ListItem, "Linus"));
    assert!(!chosen(Role::ListItem, "Grace"));
    Ok(())
}

#[test]
fn rows_that_share_a_name_are_ambiguous_and_a_blank_row_has_no_name() {
    let mut harness = Harness::mount(App::new(true), 30, 8);
    let error = select(&mut harness, Role::ListItem, "Ada").err();
    assert!(matches!(error, Some(ActError::Ambiguous(_))), "{error:?}");
    let names: Vec<_> = snapshot(&harness)
        .controls()
        .iter()
        .filter(|control| control.role == Role::ListItem)
        .map(|control| control.id.0)
        .collect();
    assert_eq!(names, [200, 201, 202, 204, 205]);
}

#[test]
fn only_the_rows_scrolled_into_view_are_controls() -> Test {
    let mut harness = Harness::mount(App::new(true), 30, 8);
    harness.component_mut().offset = 3;
    harness.advance(Duration::ZERO);
    assert!(matches!(
        select(&mut harness, Role::ListItem, "Grace"),
        Err(ActError::NotFound)
    ));
    select(&mut harness, Role::ListItem, "Margaret")?;
    assert_eq!(harness.component().row, 5);
    Ok(())
}

#[test]
fn registering_controls_does_not_change_what_is_drawn() {
    let plain = Harness::mount(App::new(false), 30, 8);
    let named = Harness::mount(App::new(true), 30, 8);
    assert_eq!(plain.rows(), named.rows());
    assert_eq!(plain.styled(), named.styled());
    assert_eq!(snapshot(&plain).controls().len(), 0);
    assert_eq!(snapshot(&named).controls().len(), 3 + 5);
}
