// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size};
use lunatui_runtime::{AccessibilityRole, ControlMetadata, HitId, LayerId};

use crate::{RenderCx, Widget};

/// Registers an existing widget as a named control without drawing extra chrome.
/// A widget's visible label supplies the name; otherwise `.name()` is required.
///
/// # Panics
///
/// Rendering panics if the selected name is empty or whitespace, in every build
/// profile and even without an attached hit map. An explicit [`name`](Self::name)
/// overrides the widget's label, so it must also be nonblank.
pub struct NamedControl<'a> {
    body: &'a dyn Widget,
    id: HitId,
    name: Option<&'a str>,
    role: AccessibilityRole,
    disabled: bool,
    layer: Option<LayerId>,
    z: u16,
    selected: bool,
    secret: bool,
}

impl<'a> NamedControl<'a> {
    /// Wraps the exact widget whose cells and geometry will be registered.
    ///
    /// # Panics
    ///
    /// Rendering the returned control panics if its selected name is empty or
    /// whitespace, even without an attached hit map. Set a nonblank `.name()`
    /// before rendering when the body has no label.
    #[must_use]
    pub const fn new(id: HitId, role: AccessibilityRole, body: &'a dyn Widget) -> Self {
        Self {
            body,
            id,
            name: None,
            role,
            disabled: false,
            layer: None,
            z: 0,
            selected: false,
            secret: false,
        }
    }

    /// An explicit name for icon-only or unlabeled controls. Never pass an input value.
    #[must_use]
    pub const fn name(mut self, name: &'a str) -> Self {
        self.name = Some(name);
        self
    }

    /// Disables activation in addition to the enclosed widget's own disabled state.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Associates the control with an overlay layer.
    #[must_use]
    pub const fn layer(mut self, layer: Option<LayerId>) -> Self {
        self.layer = layer;
        self
    }

    /// Marks this item as the chosen one in its group, for an agent's snapshot.
    #[must_use]
    pub const fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Keeps the widget's value out of an agent's snapshot, as for a password.
    #[must_use]
    pub const fn secret(mut self, secret: bool) -> Self {
        self.secret = secret;
        self
    }

    /// Exact `HitMap` depth, normally obtained from the layer stack.
    #[must_use]
    pub const fn z(mut self, z: u16) -> Self {
        self.z = z;
        self
    }
}

impl Widget for NamedControl<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let name = self.name.or_else(|| self.body.control_name()).unwrap_or("");
        assert!(
            !name.trim().is_empty(),
            "unlabeled controls require .name()"
        );
        cx.register_control(Some(self.id), area, self.z, || {
            let metadata = ControlMetadata::new(name, self.role)
                .disabled(self.disabled || self.body.control_disabled())
                .layer(self.layer)
                .checked(self.body.control_checked())
                .selected(self.selected);
            match self.body.control_value() {
                Some(value) if !self.secret => metadata.value(value),
                _ => metadata,
            }
        });
        self.body.render(area, cx);
    }

    fn measure(&self, avail: Size) -> Size {
        self.body.measure(avail)
    }
}

/// Registers each visible item of a [`Tabs`](crate::Tabs) strip, a
/// [`List`](crate::List) or a menu bar as a named control, so an agent can find and choose it.
///
/// Item `n` gets the id `first + n`, so the application maps a request's id back to
/// an index by subtracting `first`. An item with a blank name is left out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemControls {
    first: HitId,
    layer: Option<LayerId>,
    z: u16,
}

impl ItemControls {
    /// Ids start at `first` and count up by item index.
    #[must_use]
    pub const fn new(first: HitId) -> Self {
        Self {
            first,
            layer: None,
            z: 0,
        }
    }

    /// Associates the items with an overlay layer.
    #[must_use]
    pub const fn layer(mut self, layer: Option<LayerId>) -> Self {
        self.layer = layer;
        self
    }

    /// Exact `HitMap` depth, normally obtained from the layer stack.
    #[must_use]
    pub const fn z(mut self, z: u16) -> Self {
        self.z = z;
        self
    }

    /// The id of item `index`, or `None` when it would overflow.
    #[must_use]
    pub fn id(&self, index: usize) -> Option<HitId> {
        u32::try_from(index)
            .ok()
            .and_then(|index| self.first.0.checked_add(index))
            .map(HitId)
    }

    pub(crate) fn register(&self, cx: &mut RenderCx<'_>, item: &Item<'_>, area: Rect) {
        self.register_at(cx, item.index, item, area);
    }

    pub(crate) fn register_at(
        &self,
        cx: &mut RenderCx<'_>,
        index: usize,
        item: &Item<'_>,
        area: Rect,
    ) {
        if item.name.trim().is_empty() {
            return;
        }
        let Some(id) = self.id(index) else {
            return;
        };
        cx.register_control(Some(id), area, self.z, || {
            let metadata = ControlMetadata::new(item.name, item.role)
                .disabled(item.disabled)
                .layer(self.layer)
                .selected(item.selected)
                .mnemonic(item.mnemonic);
            match item.shortcut {
                Some(shortcut) => metadata.shortcut(shortcut),
                None => metadata,
            }
        });
    }
}

pub(crate) struct Item<'a> {
    pub(crate) index: usize,
    pub(crate) name: &'a str,
    pub(crate) role: AccessibilityRole,
    pub(crate) selected: bool,
    pub(crate) disabled: bool,
    pub(crate) shortcut: Option<&'a str>,
    pub(crate) mnemonic: Option<char>,
}
