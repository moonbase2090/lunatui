// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::num::NonZeroU16;

/// How a frame is drawn. Layout is the same in both modes: widgets receive a rectangle in
/// cells either way.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RenderMode {
    /// Terminal character cells.
    #[default]
    Cell,
    /// Smooth pixels: proportional text, true rounded borders and continuous charts.
    Graphite,
}

/// What an application asks for. The effective [`RenderMode`] never exceeds what the backend
/// reports, so a request for Graphite on a cell-only backend still draws cells.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ModePolicy {
    /// Graphite exactly when the backend can show it, otherwise cells.
    #[default]
    Auto,
    /// Always cells.
    Cell,
    /// Graphite when the backend can show it, otherwise cells.
    Graphite,
}

/// What a backend can show beyond cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct GraphiteCaps {
    /// Width of one cell in pixels.
    pub cell_width_px: NonZeroU16,
    /// Height of one cell in pixels.
    pub cell_height_px: NonZeroU16,
    /// Whether the backend draws proportional text itself or accepts text drawn by the app.
    pub proportional_text: bool,
    /// Whether the backend composites alpha.
    pub alpha: bool,
}

impl GraphiteCaps {
    /// Pixel output with the given cell size, no proportional text and no alpha.
    #[must_use]
    pub const fn new(cell_width_px: NonZeroU16, cell_height_px: NonZeroU16) -> Self {
        Self {
            cell_width_px,
            cell_height_px,
            proportional_text: false,
            alpha: false,
        }
    }

    /// Sets whether proportional text is available.
    #[must_use]
    pub const fn proportional_text(mut self, available: bool) -> Self {
        self.proportional_text = available;
        self
    }

    /// Sets whether alpha compositing is available.
    #[must_use]
    pub const fn alpha(mut self, available: bool) -> Self {
        self.alpha = available;
        self
    }
}

/// The modes a backend can show. A plain terminal reports [`CELL_ONLY`](Self::CELL_ONLY).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Capabilities {
    graphite: Option<GraphiteCaps>,
}

impl Capabilities {
    /// A backend that shows cells only.
    pub const CELL_ONLY: Self = Self { graphite: None };

    /// A backend that can also show Graphite mode.
    #[must_use]
    pub const fn with_graphite(caps: GraphiteCaps) -> Self {
        Self {
            graphite: Some(caps),
        }
    }

    /// The Graphite capabilities, or `None` for a cell-only backend.
    #[must_use]
    pub const fn graphite(&self) -> Option<GraphiteCaps> {
        self.graphite
    }
}

impl ModePolicy {
    /// The environment variable that overrides an application's policy, for debugging and for
    /// people who prefer cells.
    pub const ENV: &'static str = "LUNATUI_MODE";

    /// Parses `auto`, `cell` or `graphite`, ignoring case and surrounding whitespace.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "cell" => Some(Self::Cell),
            "graphite" => Some(Self::Graphite),
            _ => None,
        }
    }

    /// This policy, replaced by `value` when it parses. An unset or unrecognized value leaves
    /// the application's policy alone.
    #[must_use]
    pub fn overridden_by(self, value: Option<&str>) -> Self {
        value.and_then(Self::parse).unwrap_or(self)
    }

    /// This policy with the [`ENV`](Self::ENV) override applied.
    #[must_use]
    pub fn with_env_override(self) -> Self {
        let value = std::env::var(Self::ENV).ok();
        self.overridden_by(value.as_deref())
    }

    /// The mode to draw: the policy limited by what the backend can show. Cell mode is the
    /// floor, so this never fails.
    #[must_use]
    pub const fn resolve(self, capabilities: &Capabilities) -> RenderMode {
        match (self, capabilities.graphite) {
            (Self::Auto | Self::Graphite, Some(_)) => RenderMode::Graphite,
            _ => RenderMode::Cell,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn pixels() -> Capabilities {
        match (NonZeroU16::new(8), NonZeroU16::new(16)) {
            (Some(width), Some(height)) => {
                Capabilities::with_graphite(GraphiteCaps::new(width, height))
            }
            _ => Capabilities::CELL_ONLY,
        }
    }

    #[test]
    fn the_mode_is_the_policy_limited_by_what_the_backend_shows() {
        let cases = [
            (ModePolicy::Auto, Capabilities::CELL_ONLY, RenderMode::Cell),
            (ModePolicy::Cell, Capabilities::CELL_ONLY, RenderMode::Cell),
            (
                ModePolicy::Graphite,
                Capabilities::CELL_ONLY,
                RenderMode::Cell,
            ),
            (ModePolicy::Auto, pixels(), RenderMode::Graphite),
            (ModePolicy::Cell, pixels(), RenderMode::Cell),
            (ModePolicy::Graphite, pixels(), RenderMode::Graphite),
        ];
        for (policy, capabilities, expected) in cases {
            assert_eq!(
                policy.resolve(&capabilities),
                expected,
                "{policy:?} with {capabilities:?}"
            );
        }
    }

    #[test]
    fn defaults_draw_cells_and_leave_the_choice_to_the_backend() {
        assert_eq!(RenderMode::default(), RenderMode::Cell);
        assert_eq!(ModePolicy::default(), ModePolicy::Auto);
        assert_eq!(Capabilities::default(), Capabilities::CELL_ONLY);
        assert_eq!(Capabilities::CELL_ONLY.graphite(), None);
    }

    #[test]
    fn a_policy_parses_ignoring_case_and_surrounding_space() {
        assert_eq!(ModePolicy::parse("auto"), Some(ModePolicy::Auto));
        assert_eq!(ModePolicy::parse(" Cell\n"), Some(ModePolicy::Cell));
        assert_eq!(ModePolicy::parse("GRAPHITE"), Some(ModePolicy::Graphite));
        assert_eq!(ModePolicy::parse("pixels"), None);
        assert_eq!(ModePolicy::parse(""), None);
    }

    #[test]
    fn the_override_replaces_the_policy_only_when_it_is_recognized() {
        assert_eq!(
            ModePolicy::Auto.overridden_by(Some("cell")),
            ModePolicy::Cell
        );
        assert_eq!(
            ModePolicy::Cell.overridden_by(Some("graphite")),
            ModePolicy::Graphite
        );
        assert_eq!(
            ModePolicy::Graphite.overridden_by(Some("nonsense")),
            ModePolicy::Graphite
        );
        assert_eq!(ModePolicy::Cell.overridden_by(None), ModePolicy::Cell);
        assert_eq!(
            ModePolicy::Cell
                .overridden_by(Some("graphite"))
                .resolve(&Capabilities::CELL_ONLY),
            RenderMode::Cell,
            "an override cannot ask for more than the backend shows"
        );
    }

    #[test]
    fn graphite_caps_start_without_text_or_alpha_and_take_each_by_name() {
        let base = GraphiteCaps::new(NonZeroU16::MIN, NonZeroU16::MIN.saturating_add(1));
        assert!(!base.proportional_text && !base.alpha);
        let full = base.proportional_text(true).alpha(true);
        assert!(full.proportional_text && full.alpha);
        assert_eq!(full.cell_width_px.get(), 1);
        assert_eq!(full.cell_height_px.get(), 2);
    }
}
