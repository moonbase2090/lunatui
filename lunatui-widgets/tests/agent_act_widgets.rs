// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]
#![cfg(all(
    feature = "slider",
    feature = "number-input",
    feature = "accordion",
    feature = "tree",
    feature = "tag-input",
    feature = "calendar"
))]

use std::time::Duration;

use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_runtime::{
    AccessibilityRole as Role, ActWith, Action, ActionKind, Agent, Change, HitId, HitMap, Outcome,
    Selector,
};
use lunatui_term::Event;
use lunatui_test::{Component, Harness};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{
    Accordion, AccordionSection, Date, Monthly, NamedControl, NumberInput, Paragraph, RenderCx,
    Slider, TagInput, TagState, Tree, TreeItem, TreeState, Widget,
};

const VOLUME: HitId = HitId(1);
const COUNT: HitId = HitId(2);
const SECTION: u32 = 10;
const LABELS: HitId = HitId(3);
const DUE: HitId = HitId(4);
const NODE: u32 = 100;

fn slider() -> Slider<'static> {
    Slider::new(0.0, 100.0).step(5.0)
}

fn number() -> NumberInput<'static> {
    NumberInput::new(0.0).range(0.0, 20.0).step(1.0)
}

struct App {
    volume: f64,
    count: f64,
    accordion: Accordion,
    tree: TreeState,
    labels: TagState,
    due: Date,
    changes: Vec<Change<Outcome>>,
    agent: Agent,
    hits: HitMap,
}

impl App {
    fn new() -> Self {
        Self {
            volume: 10.0,
            count: 3.0,
            accordion: Accordion::new([
                AccordionSection::new("Network", 2),
                AccordionSection::new("Storage", 2).open(true),
                AccordionSection::new("Locked", 2).disabled(),
            ]),
            tree: TreeState::new(vec![
                TreeItem::node("src", vec![TreeItem::leaf("main.rs")]),
                TreeItem::leaf("README"),
            ]),
            labels: TagState::new(["urgent".to_owned()]),
            due: Date::UNIX_EPOCH,
            changes: Vec::new(),
            agent: Agent::new(),
            hits: HitMap::new(),
        }
    }
}

impl Component for App {
    fn update(&mut self, _: &Event) {}

    fn tick(&mut self, _: Duration) {
        while let Some(request) = self.agent.take() {
            let context = request.context();
            let change = match request.id {
                VOLUME => request.apply(
                    &mut ActWith(|action: &Action| slider().act(&mut self.volume, action)),
                    context,
                ),
                COUNT => request.apply(
                    &mut ActWith(|action: &Action| number().act(&mut self.count, action)),
                    context,
                ),
                LABELS => request.apply(&mut self.labels, context),
                DUE => request.apply(&mut self.due, context),
                HitId(id) if id >= NODE => {
                    let row = usize::try_from(id - NODE).unwrap_or(usize::MAX);
                    let Some(path) = self.tree.visible().get(row).map(|row| row.path.clone())
                    else {
                        continue;
                    };
                    request.apply(
                        &mut ActWith(|action: &Action| self.tree.act(&path, action)),
                        context,
                    )
                }
                HitId(id) => {
                    let index = usize::try_from(id - SECTION).unwrap_or(usize::MAX);
                    request.apply(
                        &mut ActWith(|action: &Action| self.accordion.act(index, action)),
                        context,
                    )
                }
            };
            self.changes.push(change);
        }
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        self.hits = HitMap::new();
        let mut cx = RenderCx::new(grid, clusters, &theme).hit_map(&mut self.hits);
        let row = |y: u16| Rect::new(area.x, area.y + y, area.width, 1);
        NamedControl::new(VOLUME, Role::Slider, &slider().value(self.volume))
            .name("Volume")
            .render(row(0), &mut cx);
        NamedControl::new(COUNT, Role::TextInput, &NumberInput::new(self.count))
            .name("Count")
            .render(row(1), &mut cx);
        let accordion_area = Rect::new(area.x, area.y + 2, area.width, 8);
        self.accordion.render(accordion_area, &mut cx);
        for (index, section) in self.accordion.layout(accordion_area).iter().enumerate() {
            let title = ["Network", "Storage", "Locked"][index];
            let id = HitId(SECTION + u32::try_from(index).unwrap_or(0));
            NamedControl::new(id, Role::Disclosure, &Paragraph::new(title))
                .name(title)
                .disabled(index == 2)
                .render(section.header, &mut cx);
        }
        let rows: Vec<(Vec<usize>, String)> = self
            .tree
            .visible()
            .iter()
            .map(|row| (row.path.clone(), row.label.to_owned()))
            .collect();
        Tree::new(&self.tree).render(Rect::new(area.x, area.y + 10, area.width, 4), &mut cx);
        for (index, (_, label)) in rows.iter().enumerate() {
            let id = HitId(NODE + u32::try_from(index).unwrap_or(0));
            NamedControl::new(id, Role::ListItem, &Paragraph::new(label.as_str()))
                .name(label)
                .render(row(10 + u16::try_from(index).unwrap_or(0)), &mut cx);
        }
        NamedControl::new(
            LABELS,
            Role::TagInput,
            &TagInput::new(self.labels.tags(), self.labels.text(), 0),
        )
        .name("Labels")
        .render(row(15), &mut cx);
        NamedControl::new(
            DUE,
            Role::Calendar,
            &Monthly::new(self.due, ()).show_month_header(Style::new()),
        )
        .name("Due")
        .render(
            Rect::new(area.x, area.y + 16, area.width.min(24), 8),
            &mut cx,
        );
    }
}

type Test = Result<(), Box<dyn std::error::Error>>;

fn mount() -> Harness<App> {
    Harness::mount(App::new(), 40, 26)
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

fn march_9() -> Date {
    Date::from_ymd(2027, 3, 9).unwrap_or(Date::UNIX_EPOCH)
}

fn set(text: &str) -> Action {
    Action::SetValue(text.to_owned())
}

#[test]
fn the_new_roles_offer_the_actions_their_widgets_take() {
    let harness = mount();
    let snapshot = Agent::snapshot(&harness.component().hits, None);
    let kinds = |name: &str| {
        snapshot
            .controls()
            .iter()
            .find(|control| control.name == name)
            .map(|control| control.actions.clone())
    };
    assert_eq!(
        kinds("Network"),
        Some(vec![ActionKind::Focus, ActionKind::Toggle])
    );
    assert_eq!(
        kinds("Labels"),
        Some(vec![
            ActionKind::Focus,
            ActionKind::SetValue,
            ActionKind::Activate
        ])
    );
    assert_eq!(
        kinds("Due"),
        Some(vec![ActionKind::Focus, ActionKind::SetValue])
    );
    assert_eq!(kinds("Locked"), Some(Vec::new()));
}

#[test]
fn an_agent_sets_a_slider_to_a_clamped_value_on_the_step() -> Test {
    let mut harness = mount();
    act(&mut harness, Role::Slider, "Volume", set("42"))?;
    assert_eq!(harness.component().volume, 40.0);
    assert_eq!(last(&harness)?, Outcome::Changed);
    act(&mut harness, Role::Slider, "Volume", set("900"))?;
    assert_eq!(harness.component().volume, 100.0);
    act(&mut harness, Role::Slider, "Volume", set("100"))?;
    assert_eq!(last(&harness)?, Outcome::Handled);
    act(&mut harness, Role::Slider, "Volume", set("loud"))?;
    assert_eq!(harness.component().volume, 100.0);
    assert_eq!(last(&harness)?, Outcome::Ignored);
    Ok(())
}

#[test]
fn an_agent_sets_a_number_field_within_its_range() -> Test {
    let mut harness = mount();
    act(&mut harness, Role::TextInput, "Count", set("7"))?;
    assert_eq!(harness.component().count, 7.0);
    assert!(harness.rows().iter().any(|row| row.contains('7')));
    act(&mut harness, Role::TextInput, "Count", set("500"))?;
    assert_eq!(harness.component().count, 20.0);
    act(&mut harness, Role::TextInput, "Count", set("-4"))?;
    assert_eq!(harness.component().count, 20.0);
    assert_eq!(last(&harness)?, Outcome::Ignored);
    act(&mut harness, Role::TextInput, "Count", set("12abc"))?;
    assert_eq!(last(&harness)?, Outcome::Ignored);
    Ok(())
}

#[test]
fn an_agent_opens_and_closes_an_accordion_section_and_the_body_follows() -> Test {
    let mut harness = mount();
    assert!(!harness.component().accordion.is_open(0));
    act(&mut harness, Role::Disclosure, "Network", Action::Toggle)?;
    assert!(harness.component().accordion.is_open(0));
    let opened = harness.rows();
    act(&mut harness, Role::Disclosure, "Network", Action::Toggle)?;
    assert!(!harness.component().accordion.is_open(0));
    assert_ne!(harness.rows(), opened);
    act(&mut harness, Role::Disclosure, "Storage", Action::Toggle)?;
    assert!(!harness.component().accordion.is_open(1));
    Ok(())
}

#[test]
fn a_disabled_accordion_section_refuses_the_agent_and_stays_closed() {
    let mut harness = mount();
    let app = harness.component_mut();
    let refused = app.agent.act(
        &app.hits,
        Selector::new(Role::Disclosure, "Locked"),
        Action::Toggle,
    );
    assert!(refused.is_err());
    harness.advance(Duration::ZERO);
    assert!(!harness.component().accordion.is_open(2));
}

#[test]
fn an_agent_opens_a_tree_branch_then_selects_a_child_that_was_hidden() -> Test {
    let mut harness = mount();
    let app = harness.component_mut();
    assert!(
        app.agent
            .act(
                &app.hits,
                Selector::new(Role::ListItem, "main.rs"),
                Action::Select
            )
            .is_err()
    );
    act(&mut harness, Role::ListItem, "src", Action::Activate)?;
    assert!(harness.component().tree.is_open(&[0]));
    act(&mut harness, Role::ListItem, "main.rs", Action::Select)?;
    assert_eq!(harness.component().tree.selected_path(), Some(&[0, 0][..]));
    act(&mut harness, Role::ListItem, "main.rs", Action::Select)?;
    assert_eq!(last(&harness)?, Outcome::Handled);
    act(&mut harness, Role::ListItem, "src", Action::Activate)?;
    assert!(!harness.component().tree.is_open(&[0]));
    assert_eq!(harness.component().tree.selected_path(), Some(&[0][..]));
    Ok(())
}

#[test]
fn an_agent_types_a_tag_and_commits_it_and_a_duplicate_is_dropped() -> Test {
    let mut harness = mount();
    act(&mut harness, Role::TagInput, "Labels", set("later"))?;
    assert_eq!(harness.component().labels.text(), "later");
    act(&mut harness, Role::TagInput, "Labels", Action::Activate)?;
    assert_eq!(harness.component().labels.tags(), ["urgent", "later"]);
    assert_eq!(harness.component().labels.text(), "");
    assert!(harness.rows().iter().any(|row| row.contains("later")));
    act(&mut harness, Role::TagInput, "Labels", set("urgent"))?;
    act(&mut harness, Role::TagInput, "Labels", Action::Activate)?;
    assert_eq!(harness.component().labels.tags(), ["urgent", "later"]);
    act(&mut harness, Role::TagInput, "Labels", Action::Activate)?;
    assert_eq!(last(&harness)?, Outcome::Handled);
    Ok(())
}

#[test]
fn an_agent_picks_a_date_and_an_impossible_date_changes_nothing() -> Test {
    let mut harness = mount();
    act(&mut harness, Role::Calendar, "Due", set("2027-03-09"))?;
    assert_eq!(harness.component().due, march_9());
    assert!(harness.rows().iter().any(|row| row.contains("March")));
    act(&mut harness, Role::Calendar, "Due", set("2027-02-30"))?;
    assert_eq!(last(&harness)?, Outcome::Ignored);
    act(&mut harness, Role::Calendar, "Due", set("9 March"))?;
    assert_eq!(harness.component().due, march_9());
    act(&mut harness, Role::Calendar, "Due", set("2027-03-09"))?;
    assert_eq!(last(&harness)?, Outcome::Handled);
    Ok(())
}
