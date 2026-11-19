// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![cfg(feature = "input")]
#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_runtime::{
    Change, ChangeContext, ChangeDetails, ChangeReason, ChangeSource, Handle, Outcome,
};
use lunatui_term::Event;
use lunatui_test::{Component, Harness};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{RenderCx, Tabs, TabsState, Widget};

struct TabApp {
    tabs: TabsState,
    refuse: bool,
    decisions: usize,
    replace_details: bool,
    last: Change<Outcome>,
}

impl TabApp {
    fn new() -> Self {
        Self {
            tabs: TabsState::new(2),
            refuse: true,
            decisions: 0,
            replace_details: false,
            last: Change::ignored(),
        }
    }
}

impl Component for TabApp {
    fn update(&mut self, event: &Event) {
        let refuse = self.refuse;
        let replace_details = self.replace_details;
        let decisions = &mut self.decisions;
        let handler: &mut dyn Handle = &mut self.tabs;
        self.last = handler.handle_with(
            event,
            ChangeContext::for_event(event).guard(&mut |details| {
                *decisions += 1;
                if replace_details {
                    *details = ChangeDetails::new(ChangeReason::Edit, ChangeSource::Agent);
                }
                details.cancel = refuse;
            }),
        );
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        Tabs::new(["Alpha", "Beta"])
            .select(self.tabs.selected())
            .highlight_style(Style::new().bg(Color::Rgb(4, 5, 6)))
            .render(area, &mut RenderCx::new(grid, clusters, &theme));
    }

    fn focus(&self) -> Option<usize> {
        Some(self.tabs.selected())
    }
}

#[cfg(feature = "select")]
mod typeahead {
    use super::*;
    use lunatui_widgets::{Select, SelectState};
    use std::time::Duration;

    const FRUIT: [&str; 2] = ["Apple", "Banana"];

    struct SearchApp {
        state: SelectState,
        refuse: bool,
        last: Change<Outcome>,
    }

    impl SearchApp {
        fn new() -> Self {
            let mut state = SelectState::new();
            state.sync(&FRUIT);
            Self {
                state,
                refuse: false,
                last: Change::ignored(),
            }
        }
    }

    impl Component for SearchApp {
        fn update(&mut self, event: &Event) {
            let refuse = self.refuse;
            self.last = self.state.handle_with(
                event,
                ChangeContext::for_event(event).guard(&mut |details| details.cancel = refuse),
            );
        }

        fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
            let theme = Theme::builtin(Variant::Dark);
            Select::new(&FRUIT, self.state.chosen())
                .render(area, &mut RenderCx::new(grid, clusters, &theme));
        }

        fn tick(&mut self, now: Duration) {
            self.state.set_time(now);
        }

        fn value(&self) -> Option<String> {
            self.state.chosen().map(|index| FRUIT[index].to_owned())
        }
    }

    #[test]
    fn vetoing_an_unmatched_character_preserves_the_query_and_its_timeout() {
        let mut harness = Harness::mount(SearchApp::new(), 20, 1);
        harness.type_text("a");
        harness.assert_value("Apple");
        harness.advance(Duration::from_millis(500));
        harness.component_mut().refuse = true;
        let before = harness.component().state.clone();
        harness.type_text("z");
        assert_eq!(harness.component().state, before);
        assert!(harness.component().last.is_canceled());
        assert_eq!(
            harness
                .component()
                .last
                .details()
                .map(ChangeDetails::reason),
            Some(ChangeReason::Typeahead)
        );
        harness.component_mut().refuse = false;
        harness.advance(Duration::from_millis(600)).type_text("b");
        harness.assert_value("Banana");
        assert!(harness.rows()[0].contains("Banana"));
    }
}

#[test]
fn refused_navigation_consumes_the_key_without_changing_the_selected_tab() {
    let mut harness = Harness::mount(TabApp::new(), 10, 1);
    let before = harness.component().tabs;
    harness.key("Right");
    harness.assert_focus(Some(0));
    harness.assert_rows(&["Alpha Beta"]);
    assert_eq!(harness.component().tabs, before);
    let result = &harness.component().last;
    assert!(result.is_canceled());
    assert!(result.is_taken());
    assert_eq!(result.value(), None);
    assert_eq!(
        result.details().map(ChangeDetails::reason),
        Some(ChangeReason::Navigate)
    );
    assert_eq!(
        result.details().map(ChangeDetails::source),
        Some(ChangeSource::Keyboard)
    );
    assert_eq!(result.details().map(|details| details.cancel), Some(true));
    assert_eq!(
        harness.grid().get(0, 0).map(|cell| cell.style().bg),
        Some(Color::Rgb(4, 5, 6))
    );

    harness.component_mut().refuse = false;
    harness.key("Right");
    harness.assert_focus(Some(1));
    assert_eq!(harness.component().last.value(), Some(&Outcome::Changed));
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(|details| details.cancel),
        Some(false)
    );
    assert_eq!(
        harness.grid().get(6, 0).map(|cell| cell.style().bg),
        Some(Color::Rgb(4, 5, 6))
    );
    assert_eq!(harness.component().decisions, 2);
}

#[test]
fn unrecognized_input_remains_available_to_the_application_without_a_guard_call() {
    let mut harness = Harness::mount(TabApp::new(), 10, 1);
    harness.key("x");
    assert!(!harness.component().last.is_taken());
    assert_eq!(harness.component().last.details(), None);
    assert_eq!(harness.component().decisions, 0);
    harness.assert_focus(Some(0));
}

#[test]
fn the_guard_can_cancel_but_cannot_relabel_the_operation_or_its_origin() {
    let mut app = TabApp::new();
    app.replace_details = true;
    let mut harness = Harness::mount(app, 10, 1);
    harness.key("Right");
    assert!(harness.component().last.is_canceled());
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(ChangeDetails::reason),
        Some(ChangeReason::Navigate)
    );
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(ChangeDetails::source),
        Some(ChangeSource::Keyboard)
    );
    harness.assert_focus(Some(0));
}
