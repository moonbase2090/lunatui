// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![cfg(feature = "nested-menu")]
#![doc = include_str!("../README.md")]

use std::{cell::Cell, rc::Rc};

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_runtime::{
    Change, ChangeContext, ChangeDetails, ChangeReason, ChangeSource, Handle, Outcome,
};
use lunatui_term::Event;
use lunatui_test::{Component, Harness};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Menu, MenuEvent, MenuItem, MenuState, RenderCx, Widget};

#[derive(Debug)]
struct Action {
    clones: Rc<Cell<usize>>,
}

impl Clone for Action {
    fn clone(&self) -> Self {
        self.clones.set(self.clones.get() + 1);
        Self {
            clones: Rc::clone(&self.clones),
        }
    }
}

struct MenuApp {
    state: MenuState<Action>,
    clones: Rc<Cell<usize>>,
    refuse: bool,
    last: Change<Outcome>,
    actions: Vec<ChangeDetails>,
}

impl MenuApp {
    fn new() -> Self {
        let clones = Rc::new(Cell::new(0));
        let mut state = MenuState::new(vec![
            MenuItem::item(
                "Run",
                Action {
                    clones: Rc::clone(&clones),
                },
            )
            .mnemonic('r'),
        ]);
        state.activate();
        Self {
            state,
            clones,
            refuse: true,
            last: Change::ignored(),
            actions: Vec::new(),
        }
    }
}

impl Component for MenuApp {
    fn update(&mut self, event: &Event) {
        let refuse = self.refuse;
        self.last = self.state.handle_with(
            event,
            ChangeContext::for_event(event).guard(&mut |details| details.cancel = refuse),
        );
        for event in self.state.drain_events() {
            if let MenuEvent::Selected { details, .. } = event {
                self.actions.push(details);
            }
        }
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        Menu::new(&self.state).render(area, &mut RenderCx::new(grid, clusters, &theme));
    }
}

#[test]
fn refusing_menu_selection_and_mnemonics_neither_clones_payloads_nor_queues_actions() {
    let mut harness = Harness::mount(MenuApp::new(), 20, 4);
    harness.key("Enter");
    assert!(harness.component().last.is_canceled());
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(ChangeDetails::reason),
        Some(ChangeReason::Activate)
    );
    harness.type_text("r");
    assert!(harness.component().last.is_canceled());
    assert!(harness.component().last.is_taken());
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(ChangeDetails::reason),
        Some(ChangeReason::Mnemonic)
    );
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(ChangeDetails::source),
        Some(ChangeSource::Keyboard)
    );
    assert_eq!(harness.component().clones.get(), 0);
    assert_eq!(harness.component().state.selected_path(), [0]);
    assert_eq!(harness.component().actions, []);
    assert!(harness.rows()[0].contains("Run"));

    harness.component_mut().refuse = false;
    harness.key("Enter");
    assert_eq!(harness.component().clones.get(), 1);
    assert_eq!(harness.component().actions.len(), 1);
    assert_eq!(
        harness.component().actions[0].reason(),
        ChangeReason::Activate
    );
    assert_eq!(
        harness.component().actions[0].source(),
        ChangeSource::Keyboard
    );
    assert!(!harness.component().actions[0].cancel);
    harness.type_text("r");
    assert_eq!(harness.component().clones.get(), 2);
    assert_eq!(harness.component().actions.len(), 2);
    assert_eq!(
        harness.component().actions[1].reason(),
        ChangeReason::Mnemonic
    );
    assert_eq!(
        harness.component().actions[1].source(),
        ChangeSource::Keyboard
    );
}
