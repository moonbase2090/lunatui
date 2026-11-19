// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![cfg(feature = "text-area")]
#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_runtime::{Change, Handle, Outcome};
use lunatui_runtime::{ChangeContext, ChangeDetails, ChangeReason, ChangeSource};
use lunatui_term::Event;
use lunatui_term::{KeyCode, KeyEvent, Modifiers, Paste};
use lunatui_test::{Component, Harness};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::TextArea;
use lunatui_widgets::{RenderCx, Widget};

fn control(ch: char) -> Event {
    Event::Key(KeyEvent::press(KeyCode::Char(ch)).modifiers(Modifiers::new().control()))
}

struct EditorApp {
    editor: TextArea,
    agent: bool,
    refuse: bool,
    install_guard: bool,
    last: Change<Outcome>,
}

impl EditorApp {
    fn new(install_guard: bool) -> Self {
        Self {
            editor: TextArea::new("safe"),
            agent: false,
            refuse: true,
            install_guard,
            last: Change::ignored(),
        }
    }
}

impl Component for EditorApp {
    fn update(&mut self, event: &Event) {
        let context = if self.agent {
            ChangeContext::new(ChangeSource::Agent)
        } else {
            ChangeContext::for_event(event)
        };
        let refuse = self.refuse;
        self.last = if self.install_guard {
            self.editor.handle_with(
                event,
                context.guard(&mut |details| {
                    details.cancel = refuse && details.source() == ChangeSource::Agent;
                }),
            )
        } else {
            self.editor.handle_with(event, context)
        };
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        self.editor
            .render(area, &mut RenderCx::new(grid, clusters, &theme));
    }

    fn value(&self) -> Option<String> {
        Some(self.editor.text().to_owned())
    }
}

#[test]
fn refused_agent_input_preserves_the_entire_editor_and_its_redo_history() {
    let mut harness = Harness::mount(EditorApp::new(true), 32, 4);
    harness.key("End").type_text(" draft").inject(&control('z'));
    harness.assert_value("safe draf");
    harness.component_mut().agent = true;
    let before = harness.component().editor.clone();
    harness.type_text("!");
    assert_eq!(harness.component().editor, before);
    harness.assert_value("safe draf");
    assert!(harness.rows().iter().any(|row| row.contains("safe draf")));
    assert!(harness.component().last.is_canceled());
    assert!(harness.component().last.is_taken());
    assert_eq!(harness.component().last.value(), None);
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(ChangeDetails::reason),
        Some(ChangeReason::Edit)
    );
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(ChangeDetails::source),
        Some(ChangeSource::Agent)
    );

    harness.inject(&control('y'));
    assert_eq!(harness.component().editor, before);
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(ChangeDetails::reason),
        Some(ChangeReason::Redo)
    );

    harness.component_mut().agent = false;
    harness.inject(&control('y'));
    harness.assert_value("safe draft");
    harness.component_mut().agent = true;
    harness.component_mut().refuse = false;
    harness.type_text("!");
    harness.assert_value("safe draft!");
    assert_eq!(harness.component().last.value(), Some(&Outcome::Changed));
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(ChangeDetails::source),
        Some(ChangeSource::Agent)
    );
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(|details| details.cancel),
        Some(false)
    );
}

#[test]
fn refusing_paste_end_keeps_a_rejected_buffer_instead_of_accepting_the_next_chunk() {
    let mut harness = Harness::mount(EditorApp::new(true), 32, 4);
    harness.dispatch(&Event::Paste(Paste::Start));
    harness.dispatch(&Event::Paste(Paste::Chunk("x".repeat(256 * 1024 + 1))));
    harness.assert_value("safe");
    harness.component_mut().agent = true;
    let before = harness.component().editor.clone();
    harness.dispatch(&Event::Paste(Paste::End));
    assert_eq!(harness.component().editor, before);
    assert!(harness.component().last.is_canceled());
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(ChangeDetails::reason),
        Some(ChangeReason::Paste)
    );

    harness.component_mut().refuse = false;
    let chunk = Event::Paste(Paste::Chunk("danger".into()));
    harness.dispatch(&chunk);
    harness.assert_value("safe");
    harness.dispatch(&Event::Paste(Paste::End));
    harness.assert_value("safe");
    assert_eq!(harness.component().last.value(), Some(&Outcome::Changed));
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(ChangeDetails::source),
        Some(ChangeSource::Agent)
    );
}

#[test]
fn accepted_guarded_and_direct_input_share_the_same_editor_interpreter() {
    let mut guarded = Harness::mount(EditorApp::new(true), 32, 4);
    let mut direct = Harness::mount(EditorApp::new(false), 32, 4);
    for event in [
        Event::Key(KeyEvent::press(KeyCode::End)),
        Event::Key(KeyEvent::press(KeyCode::Char('A'))),
        Event::Key(KeyEvent::press(KeyCode::Left)),
        Event::Key(KeyEvent::press(KeyCode::Backspace)),
    ] {
        guarded.inject(&event);
        direct.inject(&event);
        assert_eq!(guarded.component().editor, direct.component().editor);
        assert_eq!(guarded.component().last, direct.component().last);
    }
    guarded.assert_value("safA");
    direct.assert_value("safA");
}

#[test]
fn refusing_a_stationary_horizontal_move_preserves_the_preferred_column() {
    let mut harness = Harness::mount(EditorApp::new(true), 32, 4);
    harness.component_mut().editor.set_text("abc\nx");
    harness.keys("End Down");
    assert_eq!(harness.component().editor.cursor().line, 1);
    assert_eq!(harness.component().editor.cursor().grapheme, 1);
    harness.component_mut().agent = true;
    let before = harness.component().editor.clone();
    harness.key("Right");
    assert_eq!(harness.component().editor, before);
    assert!(harness.component().last.is_canceled());
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(ChangeDetails::reason),
        Some(ChangeReason::Navigate)
    );
    harness.component_mut().agent = false;
    harness.key("Up");
    assert_eq!(harness.component().editor.cursor().line, 0);
    assert_eq!(harness.component().editor.cursor().grapheme, 3);
}

#[test]
fn a_refused_agent_replacement_preserves_existing_history_and_search() {
    let mut harness = Harness::mount(EditorApp::new(true), 32, 4);
    harness
        .key("End")
        .type_text(" draft")
        .inject(&control('s'))
        .type_text("safe");
    let before = harness.component().editor.clone();
    let result = harness.component_mut().editor.set_text_with(
        "replacement",
        ChangeContext::new(ChangeSource::Agent).guard(&mut |details| details.cancel = true),
    );
    assert_eq!(harness.component().editor, before);
    assert!(result.is_taken());
    assert!(result.is_canceled());
    assert_eq!(result.value(), None);
    assert_eq!(
        result.details().map(ChangeDetails::reason),
        Some(ChangeReason::Edit)
    );
    assert_eq!(
        result.details().map(ChangeDetails::source),
        Some(ChangeSource::Agent)
    );
    let result = harness
        .component_mut()
        .editor
        .set_text_with("replacement", ChangeContext::new(ChangeSource::Agent));
    assert_eq!(result.value(), Some(&()));
    harness.advance(std::time::Duration::ZERO);
    harness.assert_value("replacement");
    assert!(harness.rows().iter().any(|row| row.contains("replacement")));
}
