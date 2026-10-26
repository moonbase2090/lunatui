// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

/// Interaction state a widget paints itself for.
///
/// Widgets map each state to semantic theme tokens, so a theme change recolors them without
/// per-widget styling.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetState {
    /// Resting appearance.
    #[default]
    Normal,
    /// The widget has input focus.
    Focused,
    /// The widget is inactive and drawn dimmed.
    Disabled,
}
