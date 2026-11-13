// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size};
use lunatui_runtime::{AccessibilityRole, ControlMetadata, HitId, LayerId};

use crate::{RenderCx, Widget};

/// Registers an existing widget as a named control without drawing extra chrome.
/// A widget's visible label supplies the name; otherwise `.name()` is required.
pub struct NamedControl<'a> {
    body: &'a dyn Widget,
    id: HitId,
    name: Option<&'a str>,
    role: AccessibilityRole,
    disabled: bool,
    layer: Option<LayerId>,
    z: u16,
}

impl<'a> NamedControl<'a> {
    /// Wraps the exact widget whose cells and geometry will be registered.
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
            ControlMetadata::new(name, self.role)
                .disabled(self.disabled || self.body.control_disabled())
                .layer(self.layer)
        });
        self.body.render(area, cx);
    }

    fn measure(&self, avail: Size) -> Size {
        self.body.measure(avail)
    }
}
