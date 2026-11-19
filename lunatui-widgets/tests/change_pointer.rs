// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![cfg(all(feature = "popover", feature = "button"))]
#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Grid, Rect, Size};
use lunatui_runtime::{
    ChangeContext, ChangeDetails, ChangeReason, ChangeSource, FocusManager, HitId, HitMap,
    LayerEvent, LayerStack,
};
use lunatui_term::{Event, MouseButton, MouseKind};
use lunatui_test::{Component, Harness};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Button, Paragraph, Popover, PopoverState, RenderCx, Widget};

const ANCHOR: Rect = Rect::new(2, 0, 6, 1);
const BUTTON: Rect = Rect::new(28, 6, 10, 1);
const OPENER: HitId = HitId(1);
const ACTION: HitId = HitId(3);

struct App {
    layers: LayerStack,
    focus: FocusManager,
    popup: PopoverState,
    refuse: bool,
    last: LayerEvent,
    clicks: usize,
    closed: usize,
    map: HitMap,
}

impl App {
    fn new() -> Self {
        let mut app = Self {
            layers: LayerStack::new(),
            focus: FocusManager::new(),
            popup: PopoverState::new(),
            refuse: true,
            last: LayerEvent::Pass,
            clicks: 0,
            closed: 0,
            map: HitMap::new(),
        };
        app.focus.sync(vec![(OPENER, ANCHOR), (ACTION, BUTTON)]);
        app.focus.focus(OPENER);
        app.popup.open(&mut app.layers, ANCHOR, Some(OPENER));
        app
    }

    fn owners(&self) -> (LayerStack, FocusManager, PopoverState) {
        (self.layers.clone(), self.focus.clone(), self.popup)
    }
}

impl Component for App {
    fn update(&mut self, event: &Event) {
        let refuse = self.refuse;
        let receipt = self.layers.handle_with(
            event,
            &mut self.focus,
            ChangeContext::for_event(event).guard(&mut |details| details.cancel = refuse),
        );
        if receipt.details().is_some() {
            self.last = receipt;
        }
        if self.popup.sync(&self.layers, &receipt).is_taken() {
            self.closed += 1;
        }
        if receipt.delivers()
            && let Event::Mouse(mouse) = event
            && mouse.kind == MouseKind::Down(MouseButton::Left)
            && self.layers.pick(&self.map, mouse.column, mouse.row) == Some(ACTION)
        {
            self.clicks += 1;
            self.focus.focus(ACTION);
        }
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        let mut cx = RenderCx::new(grid, clusters, &theme);
        self.map.clear();
        self.layers.clear_focusable();
        Button::new("Open").render(ANCHOR, &mut cx);
        Button::new("Activate").render(BUTTON, &mut cx);
        self.map.push(ACTION, BUTTON, 0);
        if self.popup.is_open() {
            let popup = Popover::new(Paragraph::new("Popup content"), ANCHOR)
                .title("Tools")
                .size(Size::new(14, 2));
            self.popup.place(&mut self.layers, popup.area(area));
            popup.render(area, &mut cx);
        }
        self.focus.sync(
            self.layers
                .focus_entries(vec![(OPENER, ANCHOR), (ACTION, BUTTON)]),
        );
    }
}

#[test]
fn a_refused_outside_press_does_not_close_or_click_through_to_the_underlying_button()
-> Result<(), Box<dyn std::error::Error>> {
    let mut harness = Harness::mount(App::new(), 40, 8);
    let before = harness.component().owners();
    let layer = harness
        .component()
        .popup
        .layer()
        .ok_or("popover has a layer")?;
    harness.click(36, 6);
    assert_eq!(harness.component().owners(), before);
    assert_eq!(harness.component().clicks, 0);
    assert_eq!(harness.component().closed, 0);
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(ChangeDetails::reason),
        Some(ChangeReason::OutsidePress(layer))
    );
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(ChangeDetails::source),
        Some(ChangeSource::Mouse)
    );
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(|details| details.cancel),
        Some(true)
    );
    assert!(harness.rows().iter().any(|row| row.contains("Tools")));
    harness.component_mut().refuse = false;
    harness.click(36, 6);
    assert!(!harness.component().popup.is_open());
    assert_eq!(harness.component().clicks, 1);
    assert_eq!(harness.component().closed, 1);
    assert_eq!(harness.component().focus.focused(), Some(ACTION));
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(|details| details.cancel),
        Some(false)
    );
    assert!(!harness.rows().iter().any(|row| row.contains("Tools")));
    Ok(())
}
