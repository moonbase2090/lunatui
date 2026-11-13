// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::fmt;

use lunatui_core::Rect;

use crate::LayerId;

use super::HitId;

/// Semantic role of an interactive control.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AccessibilityRole {
    /// An action.
    Button,
    /// One tab in a tab strip.
    Tab,
    /// One selectable list item.
    ListItem,
    /// An editable text control.
    TextInput,
    /// An on/off control.
    Checkbox,
    /// One choice in a mutually exclusive group.
    RadioButton,
    /// A control that adjusts a value along a range.
    Slider,
    /// An indicator of progress through a range.
    ProgressBar,
}

impl AccessibilityRole {
    pub(super) const fn wire_name(self) -> &'static str {
        match self {
            Self::Button => "button",
            Self::Tab => "tab",
            Self::ListItem => "list_item",
            Self::TextInput => "text_input",
            Self::Checkbox => "checkbox",
            Self::RadioButton => "radio_button",
            Self::Slider => "slider",
            Self::ProgressBar => "progress_bar",
        }
    }
}

/// A control's public identity and availability, never its input value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlMetadata {
    name: String,
    role: AccessibilityRole,
    disabled: bool,
    layer: Option<LayerId>,
}

impl ControlMetadata {
    /// Names an enabled control on the base layer.
    ///
    /// # Panics
    ///
    /// Panics if `name` is empty or only whitespace. Supply a visible label or
    /// an explicit accessible name, never the current input value.
    #[must_use]
    pub fn new(name: impl Into<String>, role: AccessibilityRole) -> Self {
        let name = name.into();
        assert!(!name.trim().is_empty(), "controls require a nonempty name");
        Self {
            name,
            role,
            disabled: false,
            layer: None,
        }
    }

    /// Whether the control is unavailable for activation.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// The owning overlay layer, or `None` for the base layer.
    #[must_use]
    pub const fn layer(mut self, layer: Option<LayerId>) -> Self {
        self.layer = layer;
        self
    }

    pub(super) fn record(&self, id: HitId, area: Rect, z: u16) -> Control<'_> {
        Control {
            id,
            area,
            z,
            name: &self.name,
            role: self.role,
            disabled: self.disabled,
            layer: self.layer,
        }
    }
}

/// One named control in the current frame. Focus is determined by comparing
/// `id` with [`crate::FocusManager::focused`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Control<'a> {
    /// Application-defined identity.
    pub id: HitId,
    /// The rendered control's terminal-cell rectangle.
    pub area: Rect,
    /// Exact depth supplied at registration.
    pub z: u16,
    /// Accessible name, independent of its input value.
    pub name: &'a str,
    /// Semantic control role.
    pub role: AccessibilityRole,
    /// Whether activation is disabled.
    pub disabled: bool,
    /// Owning overlay, or the base layer.
    pub layer: Option<LayerId>,
}

/// More than one control matched the same role, name and layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct AmbiguousControl {
    /// Number of matching records.
    pub matches: usize,
}

impl fmt::Display for AmbiguousControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "control selector is ambiguous: {} matches", self.matches)
    }
}

impl std::error::Error for AmbiguousControl {}
