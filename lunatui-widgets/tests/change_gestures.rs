// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_runtime::{Change, ChangeContext, ChangeDetails, ChangeReason, ChangeSource};
use lunatui_term::{Event, MouseButton, MouseKind};
use lunatui_test::{Component, Harness};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{
    Paragraph, RenderCx, SplitAxis, SplitDrag, SplitTree, TabMove, Tabs, TabsDrag, Widget,
};

struct TabApp {
    titles: Vec<String>,
    selected: usize,
    drag: TabsDrag,
    refuse: bool,
    last: Change<Option<TabMove>>,
}

impl TabApp {
    fn new() -> Self {
        Self {
            titles: vec!["One".into(), "Two".into(), "Three".into()],
            selected: 0,
            drag: TabsDrag::new(),
            refuse: true,
            last: Change::ignored(),
        }
    }
}

impl Component for TabApp {
    fn update(&mut self, event: &Event) {
        let Event::Mouse(mouse) = event else {
            return;
        };
        let areas = Tabs::new(self.titles.iter().cloned()).tab_areas(Rect::new(0, 0, 13, 1));
        match mouse.kind {
            MouseKind::Down(MouseButton::Left) => {
                self.drag.press(&areas, mouse.column, mouse.row);
            }
            MouseKind::Drag(MouseButton::Left) => self.drag.drag(&areas, mouse.column, mouse.row),
            MouseKind::Up(MouseButton::Left) => {
                let refuse = self.refuse;
                self.last = self.drag.release_with(
                    ChangeContext::for_event(event).guard(&mut |details| details.cancel = refuse),
                );
                if let Some(Some(movement)) = self.last.value().copied() {
                    movement.apply(&mut self.titles);
                    self.selected = movement.selection(self.selected);
                }
            }
            _ => {}
        }
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        Tabs::new(self.titles.iter().cloned())
            .select(self.selected)
            .render(area, &mut RenderCx::new(grid, clusters, &theme));
    }

    fn focus(&self) -> Option<usize> {
        Some(self.selected)
    }
}

#[test]
fn refusing_a_tab_drop_preserves_order_selection_and_the_pending_gesture() {
    let mut harness = Harness::mount(TabApp::new(), 13, 1);
    harness
        .mouse(MouseKind::Down(MouseButton::Left), 1, 0)
        .mouse(MouseKind::Drag(MouseButton::Left), 10, 0);
    let before = harness.component().drag;
    harness.mouse(MouseKind::Up(MouseButton::Left), 10, 0);
    assert_eq!(harness.component().drag, before);
    harness.assert_rows(&["One Two Three"]);
    harness.assert_focus(Some(0));
    assert!(harness.component().last.is_canceled());
    assert_eq!(harness.component().last.value(), None);
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(ChangeDetails::reason),
        Some(ChangeReason::Reorder)
    );
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(ChangeDetails::source),
        Some(ChangeSource::Mouse)
    );
    harness.component_mut().refuse = false;
    harness.mouse(MouseKind::Up(MouseButton::Left), 10, 0);
    harness.assert_rows(&["Two Three One"]);
    harness.assert_focus(Some(2));
    assert!(!harness.component().drag.is_dragging());
    assert_eq!(
        harness.component().last.value(),
        Some(&Some(TabMove { from: 0, to: 2 }))
    );
}

struct SplitApp {
    tree: SplitTree,
    drag: SplitDrag,
    refuse: bool,
    last: Change<()>,
}

impl SplitApp {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (mut tree, left) = SplitTree::new();
        tree.split(left, SplitAxis::Horizontal, 500)
            .ok_or("new pane exists")?;
        Ok(Self {
            tree,
            drag: SplitDrag::new(),
            refuse: true,
            last: Change::ignored(),
        })
    }
}

impl Component for SplitApp {
    fn update(&mut self, event: &Event) {
        let Event::Mouse(mouse) = event else {
            return;
        };
        match mouse.kind {
            MouseKind::Down(MouseButton::Left) => {
                self.drag.press(
                    &self.tree,
                    &self.tree.layout(Rect::new(0, 0, 41, 3)),
                    mouse.column,
                    mouse.row,
                );
            }
            MouseKind::Drag(MouseButton::Left) => {
                let refuse = self.refuse;
                self.last = self.drag.drag_with(
                    &mut self.tree,
                    mouse.column,
                    mouse.row,
                    ChangeContext::for_event(event).guard(&mut |details| details.cancel = refuse),
                );
            }
            _ => {}
        }
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        let mut cx = RenderCx::new(grid, clusters, &theme);
        let layout = self.tree.layout(area);
        for (index, pane) in layout.panes.iter().enumerate() {
            Paragraph::new(if index == 0 { "Left" } else { "Right" }).render(pane.rect, &mut cx);
        }
        layout.render_dividers(Style::new(), self.drag.active(), Style::new(), &mut cx);
    }
}

#[test]
fn refusing_split_resize_preserves_both_tree_and_drag_state()
-> Result<(), Box<dyn std::error::Error>> {
    let mut harness = Harness::mount(SplitApp::new()?, 41, 3);
    harness.mouse(MouseKind::Down(MouseButton::Left), 20, 1);
    let before = (harness.component().tree.clone(), harness.component().drag);
    harness.mouse(MouseKind::Drag(MouseButton::Left), 10, 1);
    assert_eq!(
        (harness.component().tree.clone(), harness.component().drag),
        before
    );
    assert_eq!(harness.rows()[0].chars().nth(20), Some('┃'));
    assert!(harness.component().last.is_canceled());
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(ChangeDetails::reason),
        Some(ChangeReason::Resize)
    );
    assert_eq!(
        harness
            .component()
            .last
            .details()
            .map(ChangeDetails::source),
        Some(ChangeSource::Mouse)
    );
    harness.component_mut().refuse = false;
    harness.mouse(MouseKind::Drag(MouseButton::Left), 10, 1);
    assert_eq!(
        harness
            .component()
            .tree
            .layout(Rect::new(0, 0, 41, 3))
            .panes[0]
            .rect
            .width,
        10
    );
    assert_eq!(harness.rows()[0].chars().nth(10), Some('┃'));
    assert_eq!(harness.component().last.value(), Some(&()));
    Ok(())
}
