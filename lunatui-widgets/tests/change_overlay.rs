// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![cfg(feature = "dialog")]
#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_runtime::{Change, ChangeContext, ChangeDetails, ChangeReason, ChangeSource};
use lunatui_runtime::{FocusManager, HitId, LayerEvent, LayerStack};
use lunatui_term::{Event, KeyCode, MouseButton, MouseKind};
use lunatui_test::{Component, Harness};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{
    Dialog, DialogClosed, DialogFocus, DialogKind, DialogOutcome, DialogState, Paragraph,
};
use lunatui_widgets::{RenderCx, Widget};

const OPENER: HitId = HitId(1);
const CONFIRM: HitId = HitId(2);

struct DialogApp {
    layers: LayerStack,
    focus: FocusManager,
    dialog: DialogState,
    refuse: bool,
    last: LayerEvent,
    answers: Vec<DialogOutcome>,
    click: Change<Option<DialogClosed>>,
}

impl DialogApp {
    fn new() -> Self {
        let mut app = Self {
            layers: LayerStack::new(),
            focus: FocusManager::new(),
            dialog: DialogState::new(),
            refuse: true,
            last: LayerEvent::Pass,
            answers: Vec::new(),
            click: Change::ignored(),
        };
        app.focus.sync(vec![(OPENER, Rect::new(0, 0, 8, 1))]);
        app.focus.focus(OPENER);
        app.dialog
            .open(DialogKind::Confirm, &mut app.layers, Some(OPENER));
        app
    }

    fn owners(&self) -> (LayerStack, FocusManager, DialogState) {
        (self.layers.clone(), self.focus.clone(), self.dialog)
    }

    fn widget(&self) -> Dialog<'static> {
        Dialog::confirm(
            "Unsaved document",
            "Keep editing until the app allows dismissal.",
        )
        .width(38)
        .state(&self.dialog)
    }
}

impl Component for DialogApp {
    fn update(&mut self, event: &Event) {
        if let Event::Key(key) = event
            && let Some(layer) = self.dialog.layer()
        {
            match key.code {
                KeyCode::F(6) => {
                    self.layers.remove(layer);
                }
                KeyCode::F(7) => {
                    self.layers.pop();
                }
                KeyCode::F(8) => {
                    self.layers.close(layer, &mut self.focus);
                }
                _ => {}
            }
        }
        let refuse = self.refuse;
        self.last = self.layers.handle_with(
            event,
            &mut self.focus,
            ChangeContext::for_event(event).guard(&mut |details| details.cancel = refuse),
        );
        let result = self.dialog.sync(&self.layers, &self.last);
        self.answers
            .extend(result.into_value().map(|closed| closed.outcome()));
        if self.last.delivers()
            && let Event::Mouse(mouse) = event
            && mouse.kind == MouseKind::Down(MouseButton::Left)
        {
            let target = self
                .widget()
                .control_at(Rect::new(0, 0, 44, 12), mouse.column, mouse.row);
            self.click = self.dialog.click_with(
                target,
                &mut self.layers,
                &mut self.focus,
                ChangeContext::for_event(event).guard(&mut |details| details.cancel = refuse),
            );
            self.answers.extend(
                self.click
                    .into_value()
                    .flatten()
                    .map(|closed| closed.outcome()),
            );
        }
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        let mut cx = RenderCx::new(grid, clusters, &theme);
        self.layers.clear_focusable();
        Paragraph::new("Editor").render(area, &mut cx);
        if self.dialog.is_open() {
            let widget = Dialog::confirm(
                "Unsaved document",
                "Keep editing until the app allows dismissal.",
            )
            .width(38)
            .state(&self.dialog);
            let panel = widget.area(area);
            self.dialog.place(&mut self.layers, panel);
            if let Some(layer) = self.dialog.layer() {
                self.layers.assign(layer, CONFIRM, panel);
            }
            widget.render(area, &mut cx);
        }
        self.focus.sync(
            self.layers
                .focus_entries(vec![(OPENER, Rect::new(0, 0, 8, 1))]),
        );
    }
}

#[test]
fn refusing_a_cancel_button_preserves_its_prior_focus_and_all_close_owners()
-> Result<(), Box<dyn std::error::Error>> {
    let mut harness = Harness::mount(DialogApp::new(), 44, 12);
    let widget = harness.component().widget();
    let target = (0..12)
        .flat_map(|row| (0..44).map(move |column| (column, row)))
        .find(|(x, y)| {
            widget.control_at(Rect::new(0, 0, 44, 12), *x, *y) == Some(DialogFocus::Cancel)
        })
        .ok_or("cancel button is drawn")?;
    let before = harness.component().owners();
    harness.click(target.0, target.1);
    assert_eq!(harness.component().owners(), before);
    assert!(harness.component().click.is_canceled());
    assert_eq!(harness.component().click.value(), None);
    assert_eq!(
        harness
            .component()
            .click
            .details()
            .map(ChangeDetails::reason),
        Some(ChangeReason::CancelButton)
    );
    assert_eq!(
        harness
            .component()
            .click
            .details()
            .map(ChangeDetails::source),
        Some(ChangeSource::Mouse)
    );
    assert_eq!(harness.component().answers, []);
    harness.component_mut().refuse = false;
    harness.click(target.0, target.1);
    assert!(!harness.component().dialog.is_open());
    assert_eq!(harness.component().answers, [DialogOutcome::Cancelled]);
    assert_eq!(harness.component().focus.focused(), Some(OPENER));
    Ok(())
}

#[test]
fn refused_escape_preserves_the_layer_slot_dialog_and_focus_then_can_be_accepted()
-> Result<(), Box<dyn std::error::Error>> {
    let mut harness = Harness::mount(DialogApp::new(), 44, 12);
    let before = harness.component().owners();
    let layer = harness
        .component()
        .dialog
        .layer()
        .ok_or("dialog has a layer")?;
    harness.key("Esc");
    assert_eq!(harness.component().owners(), before);
    assert!(!harness.component().last.delivers());
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(ChangeDetails::reason),
        Some(ChangeReason::Escape(layer))
    );
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(ChangeDetails::source),
        Some(ChangeSource::Keyboard)
    );
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(|details| details.cancel),
        Some(true)
    );
    assert_eq!(harness.component().answers, []);
    assert!(
        harness
            .rows()
            .iter()
            .any(|row| row.contains("Unsaved document"))
    );

    harness.component_mut().refuse = false;
    harness.key("Esc");
    assert!(!harness.component().dialog.is_open());
    assert!(harness.component().layers.is_empty());
    assert_eq!(harness.component().focus.focused(), Some(OPENER));
    assert_eq!(harness.component().answers, [DialogOutcome::Cancelled]);
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(ChangeDetails::reason),
        Some(ChangeReason::Escape(layer))
    );
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(|details| details.cancel),
        Some(false)
    );
    assert!(harness.rows()[0].starts_with("Editor"));
    Ok(())
}

#[test]
fn application_layer_removal_cancels_the_dialog_once() {
    for key in ["F6", "F7", "F8"] {
        let mut harness = Harness::mount(DialogApp::new(), 44, 12);
        harness.key(key);
        assert!(!harness.component().dialog.is_open());
        assert!(harness.component().layers.is_empty());
        assert_eq!(harness.component().answers, [DialogOutcome::Cancelled]);
        assert_eq!(harness.component().focus.focused(), Some(OPENER));
        assert!(
            harness
                .rows()
                .iter()
                .all(|row| !row.contains("Unsaved document"))
        );
        harness.key(key);
        assert_eq!(harness.component().answers, [DialogOutcome::Cancelled]);
    }
}
