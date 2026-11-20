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
    /// A closed list of options that opens to choose one.
    ComboBox,
    /// One entry of a menu bar or a menu panel, a leaf or a group that opens a submenu.
    MenuItem,
}

impl AccessibilityRole {
    pub(crate) const fn wire_name(self) -> &'static str {
        match self {
            Self::Button => "button",
            Self::Tab => "tab",
            Self::ListItem => "list_item",
            Self::TextInput => "text_input",
            Self::Checkbox => "checkbox",
            Self::RadioButton => "radio_button",
            Self::Slider => "slider",
            Self::ProgressBar => "progress_bar",
            Self::ComboBox => "combo_box",
            Self::MenuItem => "menu_item",
        }
    }
}

/// A control's public identity and availability.
///
/// The optional value, checked, selected and invalid facets are for
/// [`crate::Agent`] only. The accessibility export never writes them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlMetadata {
    name: String,
    role: AccessibilityRole,
    disabled: bool,
    layer: Option<LayerId>,
    value: Option<String>,
    checked: Option<bool>,
    selected: bool,
    invalid: bool,
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
            value: None,
            checked: None,
            selected: false,
            invalid: false,
        }
    }

    /// The current value an agent may read, such as the text in an input or the chosen option.
    /// It is never part of the accessibility export. Skip it for secrets.
    #[must_use]
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Whether an on/off control is on.
    #[must_use]
    pub const fn checked(mut self, checked: Option<bool>) -> Self {
        self.checked = checked;
        self
    }

    /// Whether this item is the chosen one in its group.
    #[must_use]
    pub const fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Whether the control shows a validation error.
    #[must_use]
    pub const fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
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
            value: self.value.as_deref(),
            checked: self.checked,
            selected: self.selected,
            invalid: self.invalid,
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
    /// The value an agent may read. Never exported as accessibility JSON.
    pub value: Option<&'a str>,
    /// Whether an on/off control is on.
    pub checked: Option<bool>,
    /// Whether this item is the chosen one in its group.
    pub selected: bool,
    /// Whether the control shows a validation error.
    pub invalid: bool,
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
