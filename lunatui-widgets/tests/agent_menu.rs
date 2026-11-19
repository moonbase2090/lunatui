// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![cfg(feature = "nested-menu")]
#![doc = include_str!("../README.md")]

use std::time::Duration;

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_runtime::{
    AccessibilityRole as Role, ActError, Action, Agent, ChangeContext, ChangeDetails, ChangeReason,
    ChangeSource, ControlState, Handle, HitId, HitMap, Selector, Snapshot,
};
use lunatui_term::Event;
use lunatui_test::{Component, Harness};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{ItemControls, Menu, MenuEvent, MenuItem, MenuState, RenderCx, Widget};

const FIRST: u32 = 100;

fn items() -> Vec<MenuItem<&'static str>> {
    vec![
        MenuItem::group(
            "File",
            vec![
                MenuItem::item("New", "new"),
                MenuItem::item("Open", "open"),
                MenuItem::group(
                    "Open recent",
                    vec![
                        MenuItem::item("file_1.txt", "recent-1"),
                        MenuItem::item("file_2.txt", "recent-2"),
                    ],
                ),
                MenuItem::item("Save as", "save-as").disabled(true),
            ],
        ),
        MenuItem::group(
            "Edit",
            vec![
                MenuItem::item("Copy", "copy"),
                MenuItem::item("Cut", "cut"),
                MenuItem::item("Paste", "paste"),
            ],
        ),
        MenuItem::group("Tools", vec![MenuItem::item("Run", "run")]).disabled(true),
        MenuItem::item("Help", "help"),
    ]
}

struct App {
    state: MenuState<&'static str>,
    controls: bool,
    refuse: bool,
    chosen: Vec<(&'static str, ChangeDetails)>,
    agent: Agent,
    hits: HitMap,
}

impl App {
    fn new(controls: bool) -> Self {
        Self::with(MenuState::new(items()), controls)
    }

    fn with(state: MenuState<&'static str>, controls: bool) -> Self {
        Self {
            state,
            controls,
            refuse: false,
            chosen: Vec::new(),
            agent: Agent::new(),
            hits: HitMap::new(),
        }
    }

    fn collect(&mut self) {
        for event in self.state.drain_events() {
            if let MenuEvent::Selected { value, details } = event {
                self.chosen.push((value, details));
            }
        }
    }
}

impl Component for App {
    fn update(&mut self, event: &Event) {
        self.state
            .handle_with(event, ChangeContext::for_event(event));
        self.collect();
    }

    fn tick(&mut self, _: Duration) {
        while let Some(request) = self.agent.take() {
            let Some(index) = request.id.0.checked_sub(FIRST) else {
                continue;
            };
            let index = usize::try_from(index).unwrap_or(usize::MAX);
            let refuse = self.refuse;
            let mut guard = |details: &mut ChangeDetails| details.cancel = refuse;
            let context = ChangeContext::new(ChangeSource::Agent).guard(&mut guard);
            match request.action {
                Action::Select => {
                    self.state.highlight_with(index, context);
                }
                Action::Activate => {
                    self.state.activate_with(index, context);
                }
                _ => {}
            }
        }
        self.collect();
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        self.hits = HitMap::new();
        let mut cx = RenderCx::new(grid, clusters, &theme).hit_map(&mut self.hits);
        let menu = Menu::new(&self.state);
        let menu = if self.controls {
            menu.controls(ItemControls::new(HitId(FIRST)))
        } else {
            menu
        };
        menu.render(area, &mut cx);
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

fn select(harness: &mut Harness<App>, name: &str) -> Result<(), ActError> {
    act(harness, Role::MenuItem, name, Action::Select)
}

fn activate(harness: &mut Harness<App>, name: &str) -> Result<(), ActError> {
    act(harness, Role::MenuItem, name, Action::Activate)
}

fn snapshot(harness: &Harness<App>) -> Snapshot {
    Agent::snapshot(&harness.component().hits, None)
}

fn names(harness: &Harness<App>) -> Vec<String> {
    snapshot(harness)
        .controls()
        .iter()
        .map(|control| control.name.clone())
        .collect()
}

fn mount(controls: bool) -> Harness<App> {
    Harness::mount(App::new(controls), 40, 10)
}

#[test]
fn an_agent_walks_into_a_nested_panel_and_chooses_a_leaf_by_name() -> Test {
    let mut harness = mount(true);
    assert_eq!(names(&harness), ["File", "Edit", "Tools", "Help"]);

    select(&mut harness, "File")?;
    assert_eq!(
        names(&harness),
        [
            "File",
            "Edit",
            "Tools",
            "Help",
            "New",
            "Open",
            "Open recent",
            "Save as"
        ]
    );
    select(&mut harness, "Open recent")?;
    assert!(names(&harness).contains(&"file_2.txt".to_owned()));
    activate(&mut harness, "file_2.txt")?;

    let app = harness.component();
    assert_eq!(app.chosen.len(), 1);
    let (value, details) = &app.chosen[0];
    assert_eq!(*value, "recent-2");
    assert_eq!(details.source(), ChangeSource::Agent);
    assert_eq!(details.reason(), ChangeReason::Activate);
    Ok(())
}

#[test]
fn the_agent_and_the_keyboard_reach_the_same_state_and_the_same_picture() -> Test {
    let mut agent = mount(true);
    select(&mut agent, "File")?;
    select(&mut agent, "Open recent")?;
    activate(&mut agent, "file_2.txt")?;

    let mut keyboard = mount(true);
    keyboard.component_mut().state.activate();
    keyboard.keys("Down Down Down Right Down Enter");

    assert_eq!(
        agent.component().state.selected_path(),
        keyboard.component().state.selected_path()
    );
    assert_eq!(agent.component().state.selected_path(), [0, 2, 1]);
    let values = |harness: &Harness<App>| -> Vec<_> {
        harness.component().chosen.iter().map(|(v, _)| *v).collect()
    };
    assert_eq!(values(&agent), ["recent-2"]);
    assert_eq!(values(&keyboard), ["recent-2"]);
    assert_eq!(agent.rows(), keyboard.rows());
    assert_eq!(agent.styled(), keyboard.styled());
    Ok(())
}

#[test]
fn choosing_a_group_opens_it_as_enter_does_and_selecting_only_highlights_it() -> Test {
    let mut harness = mount(true);
    select(&mut harness, "File")?;
    select(&mut harness, "Open recent")?;
    assert_eq!(harness.component().state.selected_path(), [0, 2]);
    assert!(harness.rows().iter().any(|row| row.contains("file_1.txt")));

    activate(&mut harness, "Open recent")?;
    assert_eq!(harness.component().state.selected_path(), [0, 2, 0]);
    assert_eq!(harness.component().chosen, []);

    let snapshot = snapshot(&harness);
    let chosen = |name| {
        snapshot
            .find(Selector::new(Role::MenuItem, name))
            .ok()
            .flatten()
            .is_some_and(|control| control.states.contains(&ControlState::Selected))
    };
    assert!(chosen("File"));
    assert!(chosen("Open recent"));
    assert!(chosen("file_1.txt"));
    assert!(!chosen("file_2.txt"));
    assert!(!chosen("Edit"));
    Ok(())
}

#[test]
fn only_the_open_panels_are_controls_and_closing_one_removes_its_rows() -> Test {
    let mut harness = mount(true);
    assert!(matches!(
        select(&mut harness, "Copy"),
        Err(ActError::NotFound)
    ));
    select(&mut harness, "Edit")?;
    select(&mut harness, "Copy")?;
    assert!(matches!(
        select(&mut harness, "New"),
        Err(ActError::NotFound)
    ));
    select(&mut harness, "File")?;
    assert!(matches!(
        select(&mut harness, "Copy"),
        Err(ActError::NotFound)
    ));
    select(&mut harness, "New")?;
    Ok(())
}

#[test]
fn an_id_names_the_same_item_whichever_panels_are_open() -> Test {
    let mut harness = mount(true);
    let id = |harness: &Harness<App>, name: &str| {
        snapshot(harness)
            .find(Selector::new(Role::MenuItem, name))
            .ok()
            .flatten()
            .map(|control| control.id.0 - FIRST)
    };
    assert_eq!(id(&harness, "Edit"), Some(7));
    assert_eq!(id(&harness, "Help"), Some(13));
    select(&mut harness, "File")?;
    select(&mut harness, "Open recent")?;
    assert_eq!(id(&harness, "file_2.txt"), Some(5));
    assert_eq!(id(&harness, "Edit"), Some(7));
    assert_eq!(id(&harness, "Help"), Some(13));
    select(&mut harness, "Edit")?;
    assert_eq!(id(&harness, "Cut"), Some(9));
    let state = &harness.component().state;
    for (index, path) in [
        (0, vec![0]),
        (5, vec![0, 2, 1]),
        (6, vec![0, 3]),
        (12, vec![2, 0]),
        (13, vec![3]),
    ] {
        assert_eq!(state.path_of(index), Some(path), "{index}");
    }
    assert_eq!(state.path_of(14), None);
    Ok(())
}

#[test]
fn a_disabled_item_is_a_disabled_control_and_cannot_be_chosen() -> Test {
    let mut harness = mount(true);
    assert!(matches!(
        select(&mut harness, "Tools"),
        Err(ActError::Disabled)
    ));
    assert!(matches!(
        activate(&mut harness, "Tools"),
        Err(ActError::Disabled)
    ));
    select(&mut harness, "File")?;
    assert!(matches!(
        activate(&mut harness, "Save as"),
        Err(ActError::Disabled)
    ));
    let disabled = snapshot(&harness)
        .find(Selector::new(Role::MenuItem, "Save as"))
        .ok()
        .flatten()
        .is_some_and(|control| control.states.contains(&ControlState::Disabled));
    assert!(disabled);
    assert_eq!(harness.component().chosen, []);
    assert_eq!(harness.component().state.selected_path(), [0]);
    Ok(())
}

#[test]
fn an_item_inside_a_disabled_group_is_never_reached_by_its_id() {
    let mut state = MenuState::new(items());
    let refuse = ChangeContext::new(ChangeSource::Agent);
    assert!(!state.highlight_with(12, refuse).is_taken());
    assert_eq!(state.selected_path(), [0_usize; 0]);
    let refuse = ChangeContext::new(ChangeSource::Agent);
    assert!(!state.activate_with(12, refuse).is_taken());
    assert!(state.drain_events().next().is_none());
}

#[test]
fn a_refused_agent_action_leaves_the_path_and_the_queue_as_they_were() -> Test {
    let mut harness = mount(true);
    select(&mut harness, "File")?;
    harness.component_mut().refuse = true;
    select(&mut harness, "Open recent")?;
    activate(&mut harness, "Open")?;
    assert_eq!(harness.component().state.selected_path(), [0]);
    assert_eq!(harness.component().chosen, []);

    harness.component_mut().refuse = false;
    activate(&mut harness, "Open")?;
    assert_eq!(harness.component().chosen.len(), 1);
    assert_eq!(harness.component().chosen[0].0, "open");
    Ok(())
}

#[test]
fn panels_that_show_the_same_name_make_act_ambiguous() -> Test {
    let state = MenuState::new(vec![MenuItem::group(
        "File",
        vec![
            MenuItem::item("Close", "close"),
            MenuItem::group("More", vec![MenuItem::item("Close", "close-all")]),
        ],
    )]);
    let mut harness = Harness::mount(App::with(state, true), 40, 10);
    select(&mut harness, "File")?;
    select(&mut harness, "More")?;
    let error = select(&mut harness, "Close").err();
    assert!(matches!(error, Some(ActError::Ambiguous(_))), "{error:?}");
    Ok(())
}

#[test]
fn a_menu_item_takes_select_and_activate_and_nothing_else() -> Test {
    let mut harness = mount(true);
    let snapshot = snapshot(&harness);
    let file = snapshot
        .find(Selector::new(Role::MenuItem, "File"))
        .ok()
        .flatten()
        .ok_or("File is a control")?;
    let kinds: Vec<_> = file.actions.iter().map(|kind| kind.name()).collect();
    assert_eq!(kinds, ["focus", "select", "activate"]);
    assert!(matches!(
        act(&mut harness, Role::MenuItem, "File", Action::Toggle),
        Err(ActError::Unsupported(_))
    ));
    Ok(())
}

#[test]
fn registering_controls_does_not_change_what_is_drawn() {
    let (mut plain, mut named) = (mount(false), mount(true));
    assert_eq!(plain.rows(), named.rows());
    assert_eq!(plain.styled(), named.styled());
    plain.component_mut().state.activate();
    named.component_mut().state.activate();
    for key in ["Down", "Down Down Right", "Down"] {
        plain.keys(key);
        named.keys(key);
        assert_eq!(plain.rows(), named.rows(), "{key}");
        assert_eq!(plain.styled(), named.styled(), "{key}");
    }
    assert_eq!(snapshot(&plain).controls().len(), 0);
    assert_eq!(snapshot(&named).controls().len(), 4 + 4 + 2);
}
