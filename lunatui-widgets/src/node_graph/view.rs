// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use super::NodeGraph;
use super::canvas::{Glyph, Role};
use crate::render_cx::clip_to_coordinates;
use crate::{RenderCx, Widget, WidgetState};
use lunatui_core::{Rect, Size, Style, cluster_display_width};
use lunatui_theme::Token;

/// A themed viewport into a [`NodeGraph`]. Selection and scrolling are application state.
#[derive(Clone, Copy, Debug)]
pub struct NodeGraphView<'a> {
    pub(super) graph: &'a NodeGraph,
    selected: Option<usize>,
    scroll: (u16, u16),
    state: WidgetState,
    highlight: Option<Style>,
}

impl<'a> NodeGraphView<'a> {
    pub(super) const fn new(graph: &'a NodeGraph) -> Self {
        Self {
            graph,
            selected: None,
            scroll: (0, 0),
            state: WidgetState::Normal,
            highlight: None,
        }
    }

    /// Highlights this node's border and title. Missing indices select nothing.
    #[must_use]
    pub const fn selected(mut self, node: Option<usize>) -> Self {
        self.selected = node;
        self
    }

    /// Scrolls in graph cells, clamped to the visible extent at draw time.
    #[must_use]
    pub const fn scroll(mut self, x: u16, y: u16) -> Self {
        self.scroll = (x, y);
        self
    }

    /// Sets the interaction state. Disabled views are dim and do not show selection.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Replaces the selected border style, for example with an eased theme-color transition.
    #[must_use]
    pub const fn highlight_style(mut self, style: Style) -> Self {
        self.highlight = Some(style);
        self
    }

    /// The largest scroll offset that still fills this viewport.
    #[must_use]
    pub const fn max_scroll(&self, area: Rect) -> (u16, u16) {
        (
            self.graph.size.width.saturating_sub(area.width),
            self.graph.size.height.saturating_sub(area.height),
        )
    }

    /// Finds the node at a terminal cell after scrolling, or `None` outside the viewport.
    /// Disabled views do not accept hits.
    #[must_use]
    pub fn node_at(&self, area: Rect, x: u16, y: u16) -> Option<usize> {
        let area = clip_to_coordinates(area);
        if self.state == WidgetState::Disabled
            || x < area.x
            || y < area.y
            || x - area.x >= area.width
            || y - area.y >= area.height
        {
            return None;
        }
        let max = self.max_scroll(area);
        let column = u32::from(x - area.x) + u32::from(self.scroll.0.min(max.0));
        let row = u32::from(y - area.y) + u32::from(self.scroll.1.min(max.1));
        self.graph.placements.iter().position(|rect| {
            column >= u32::from(rect.x)
                && column < u32::from(rect.x) + u32::from(rect.width)
                && row >= u32::from(rect.y)
                && row < u32::from(rect.y) + u32::from(rect.height)
        })
    }

    pub(super) fn style(self, role: Role, cx: &RenderCx<'_>) -> Style {
        let raised = matches!(role, Role::Border(_) | Role::Title(_) | Role::Body);
        let background = cx.token(if raised {
            Token::Surface1
        } else {
            Token::Surface0
        });
        let foreground = cx.token(match role {
            Role::Edge => Token::Accent,
            Role::Border(_) | Role::Title(_) => Token::TextMuted,
            Role::Body | Role::Ground => Token::TextPrimary,
        });
        let mut style = Style::new().fg(foreground).bg(background);
        if let Role::Border(node) | Role::Title(node) = role
            && self.selected == Some(node)
            && self.state != WidgetState::Disabled
        {
            style = self.highlight.unwrap_or_else(|| {
                style
                    .fg(cx.token(Token::FocusRing))
                    .with_attrs(style.attrs.set_bold(true))
            });
        }
        if self.state == WidgetState::Disabled {
            style.attrs = style.attrs.set_dim(true);
        }
        style
    }
}

impl Widget for NodeGraphView<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let area = clip_to_coordinates(area);
        if area.width == 0 || area.height == 0 {
            return;
        }
        cx.fill(area, self.style(Role::Ground, cx), ' ');
        let max = self.max_scroll(area);
        let (left, top) = (self.scroll.0.min(max.0), self.scroll.1.min(max.1));
        #[cfg(feature = "pixel-node-graph")]
        let pixels = super::pixel::draw(self, area, (left, top), cx);
        let canvas = &self.graph.canvas;
        for row in 0..area.height.min(self.graph.size.height) {
            for column in 0..area.width.min(self.graph.size.width) {
                let x = left + column;
                let y = top + row;
                let Some(cell) = canvas
                    .cells
                    .get(usize::from(y) * usize::from(self.graph.size.width) + usize::from(x))
                else {
                    continue;
                };
                #[cfg(feature = "pixel-node-graph")]
                if pixels && matches!(cell.role, Role::Edge | Role::Border(_)) {
                    continue;
                }
                let mut buffer = [0_u8; 4];
                let text: &str = match cell.glyph {
                    Glyph::Char(c) => c.encode_utf8(&mut buffer),
                    Glyph::Cluster(index) => &canvas.clusters[index],
                    Glyph::Tail if column == 0 => " ",
                    Glyph::Tail => continue,
                };
                let text = if cluster_display_width(text) > 1 && column + 1 >= area.width {
                    " "
                } else {
                    text
                };
                cx.put_str(
                    area.x + column,
                    area.y + row,
                    text,
                    self.style(cell.role, cx),
                );
            }
        }
    }

    fn measure(&self, avail: Size) -> Size {
        self.graph.size.clamp(avail)
    }
}
