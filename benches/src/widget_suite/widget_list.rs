// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::sync::OnceLock;

use lunatui_core::{Attrs, ClusterTable, Grid, Rect, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    Block, BorderType, Paragraph, RenderCx, Widget, WidgetList, WidgetListState,
};
use ratatui::widgets::{Block as RatBlock, BorderType as RatBorder, Paragraph as RatParagraph};
use ratatui::{
    buffer::Buffer, layout::Rect as RatRect, style::Modifier, widgets::Widget as RatWidget,
};
use tui_widget_list::{ListBuilder, ListState, ListView};

/// Items in the feed. One selection step takes [`STEP`] ticks.
pub const ITEMS: usize = 21;
const STEP: u32 = 10;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Section,
    Message,
    Progress,
    Alert,
}

const KINDS: [Kind; 7] = [
    Kind::Section,
    Kind::Message,
    Kind::Message,
    Kind::Progress,
    Kind::Alert,
    Kind::Message,
    Kind::Progress,
];
const TITLES: [&str; 7] = [
    "Today",
    "Release train is green",
    "Review requested on the layout pass",
    "Nightly benchmarks",
    "Deploy blocked",
    "Docs build finished",
    "Cache warm-up",
];
const DETAILS: [&str; 7] = [
    "",
    "All 15 checks passed on main.",
    "Two files changed, one comment open.",
    "",
    "Missing sign-off for the lockfile change.",
    "Site and API docs are published.",
    "",
];
const META: [&str; 7] = [
    "",
    "2 min ago",
    "14 min ago",
    "",
    "Needs an owner",
    "1 h ago",
    "",
];

fn kind(index: usize) -> Kind {
    KINDS[index % KINDS.len()]
}

fn height(index: usize) -> u16 {
    match kind(index) {
        Kind::Section => 1,
        Kind::Message => 3,
        Kind::Progress => 2,
        Kind::Alert => 4,
    }
}

fn selected(tick: u32) -> usize {
    usize::try_from(tick / STEP).unwrap_or(0) % ITEMS
}

fn bar(index: usize, width: u16) -> String {
    let width = usize::from(width);
    let filled = (index * 11) % 100 * width / 100;
    let mut text = "█".repeat(filled);
    text.push_str(&"░".repeat(width - filled));
    text
}

struct Styles {
    card: Style,
    pick: Style,
    dim: Style,
    dim_pick: Style,
    accent: Style,
    accent_pick: Style,
    alert: Style,
    alert_dim: Style,
    page: Style,
    panel_border: Style,
}

fn ink(theme: &Theme, fg: Token, bg: Token) -> Style {
    Style::new().fg(theme.resolve(fg)).bg(theme.resolve(bg))
}

impl Styles {
    fn new(theme: &Theme) -> Self {
        let bold = Attrs::new().set_bold(true);
        Self {
            card: ink(theme, Token::TextPrimary, Token::Surface1),
            pick: ink(theme, Token::TextStrong, Token::FocusFill).with_attrs(bold),
            dim: ink(theme, Token::TextMuted, Token::Surface1),
            dim_pick: ink(theme, Token::TextMutedOnFocus, Token::FocusFill).with_attrs(bold),
            accent: ink(theme, Token::Accent, Token::Surface1),
            accent_pick: ink(theme, Token::Ok, Token::FocusFill).with_attrs(bold),
            alert: ink(theme, Token::Fail, Token::Surface1),
            alert_dim: ink(theme, Token::TextMuted, Token::Surface1),
            page: ink(theme, Token::TextPrimary, Token::Surface0),
            panel_border: ink(theme, Token::Border, Token::Surface1),
        }
    }

    // A selected row is bold. The baseline patches styles onto the row it filled first, so the
    // alert's styles carry the same bold explicitly on both sides.
    fn alert(&self, picked: bool) -> Style {
        let bold = Attrs::new().set_bold(picked);
        self.alert.with_attrs(bold)
    }

    fn alert_dim(&self, picked: bool) -> Style {
        let bold = Attrs::new().set_bold(picked);
        self.alert_dim.with_attrs(bold)
    }

    fn row(&self, picked: bool) -> Style {
        if picked { self.pick } else { self.card }
    }

    fn dim(&self, picked: bool) -> Style {
        if picked { self.dim_pick } else { self.dim }
    }

    fn accent(&self, picked: bool) -> Style {
        if picked {
            self.accent_pick
        } else {
            self.accent
        }
    }
}

fn panel(area: Rect) -> Rect {
    let width = area.width.saturating_sub(4).min(60);
    let height = area.height.saturating_sub(2).min(30);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

fn list_area(panel: Rect) -> Rect {
    Rect::new(
        panel.x + 2,
        panel.y + 3,
        panel.width - 4,
        panel.height.saturating_sub(5),
    )
}

fn rat_rect(area: Rect) -> RatRect {
    RatRect::new(area.x, area.y, area.width, area.height)
}

fn rat_style(style: Style) -> ratatui::style::Style {
    let mut result = ratatui::style::Style::new()
        .fg(super::rat_color(style.fg))
        .bg(super::rat_color(style.bg));
    if style.attrs.bold() {
        result = result.add_modifier(Modifier::BOLD);
    }
    if style.attrs.dim() {
        result = result.add_modifier(Modifier::DIM);
    }
    result
}

fn text_native(cx: &mut RenderCx<'_>, area: Rect, text: &str, style: Style) {
    Block::new().style(style).render(area, cx);
    Paragraph::new(text).style(style).render(area, cx);
}

fn draw_native(styles: &Styles, index: usize, area: Rect, picked: bool, cx: &mut RenderCx<'_>) {
    let which = index % KINDS.len();
    let row = styles.row(picked);
    Block::new().style(row).render(area, cx);
    let line = |y: u16| Rect::new(area.x, area.y + y, area.width, 1);
    match kind(index) {
        Kind::Section => {
            text_native(cx, line(0), TITLES[which], styles.accent(picked));
        }
        Kind::Message => {
            text_native(cx, line(0), TITLES[which], row);
            text_native(cx, line(1), DETAILS[which], styles.dim(picked));
            text_native(cx, line(2), META[which], styles.dim(picked));
        }
        Kind::Progress => {
            text_native(cx, line(0), TITLES[which], row);
            text_native(
                cx,
                line(1),
                bar(index, area.width).as_str(),
                styles.accent(picked),
            );
        }
        Kind::Alert => {
            Block::bordered()
                .border_type(BorderType::Rounded)
                .style(styles.alert(picked))
                .border_style(styles.alert(picked))
                .render(area, cx);
            let inner = Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), 1);
            text_native(cx, inner, TITLES[which], styles.alert(picked));
            text_native(
                cx,
                Rect::new(inner.x, inner.y + 1, inner.width, 1),
                DETAILS[which],
                styles.alert_dim(picked),
            );
        }
    }
}

/// Renders the native widget-list scene from the fixture's data and supplied theme.
pub fn native(grid: &mut Grid, clusters: &mut ClusterTable, theme: &Theme, area: Rect, tick: u32) {
    let styles = Styles::new(theme);
    let mut cx = RenderCx::new(grid, clusters, theme);
    Block::new().style(styles.page).render(area, &mut cx);
    let panel = panel(area);
    if panel.width < 12 || panel.height < 10 {
        return;
    }
    Block::bordered()
        .border_type(BorderType::Rounded)
        .style(styles.card)
        .border_style(styles.panel_border)
        .render(panel, &mut cx);
    let head = Rect::new(panel.x + 2, panel.y + 1, panel.width - 4, 1);
    text_native(&mut cx, head, "Activity feed", styles.accent);
    let state = WidgetListState {
        selected: Some(selected(tick)),
        offset: 0,
        skip: 0,
    };
    WidgetList::new(
        ITEMS,
        |index, _width| height(index),
        |index, area, picked, cx: &mut RenderCx<'_>| draw_native(&styles, index, area, picked, cx),
    )
    .style(styles.card)
    .highlight_style(styles.pick)
    .state(state)
    .render(list_area(panel), &mut cx);
}

struct Item<'a> {
    index: usize,
    picked: bool,
    styles: &'a Styles,
}

impl RatWidget for Item<'_> {
    fn render(self, area: RatRect, buffer: &mut Buffer) {
        let styles = self.styles;
        let which = self.index % KINDS.len();
        let row = styles.row(self.picked);
        RatBlock::new().style(rat_style(row)).render(area, buffer);
        let line = |y: u16| RatRect::new(area.x, area.y + y, area.width, 1);
        let text = |value: &str, style: Style, y: u16, buffer: &mut Buffer| {
            if y < area.height {
                RatParagraph::new(value)
                    .style(rat_style(style))
                    .render(line(y), buffer);
            }
        };
        match kind(self.index) {
            Kind::Section => text(TITLES[which], styles.accent(self.picked), 0, buffer),
            Kind::Message => {
                text(TITLES[which], row, 0, buffer);
                text(DETAILS[which], styles.dim(self.picked), 1, buffer);
                text(META[which], styles.dim(self.picked), 2, buffer);
            }
            Kind::Progress => {
                text(TITLES[which], row, 0, buffer);
                text(
                    &bar(self.index, area.width),
                    styles.accent(self.picked),
                    1,
                    buffer,
                );
            }
            Kind::Alert => {
                RatBlock::bordered()
                    .border_type(RatBorder::Rounded)
                    .style(rat_style(styles.alert(self.picked)))
                    .border_style(rat_style(styles.alert(self.picked)))
                    .render(area, buffer);
                for (y, value, style) in [
                    (1, TITLES[which], styles.alert(self.picked)),
                    (2, DETAILS[which], styles.alert_dim(self.picked)),
                ] {
                    if y < area.height {
                        RatParagraph::new(value).style(rat_style(style)).render(
                            RatRect::new(area.x + 2, area.y + y, area.width.saturating_sub(4), 1),
                            buffer,
                        );
                    }
                }
            }
        }
    }
}

/// Renders the same scene with tui-widget-list, using native-equivalent styles.
pub fn baseline_themed(buffer: &mut Buffer, area: RatRect, theme: &Theme, tick: u32) {
    let styles = Styles::new(theme);
    RatBlock::new()
        .style(rat_style(styles.page))
        .render(area, buffer);
    let panel = panel(Rect::new(area.x, area.y, area.width, area.height));
    if panel.width < 12 || panel.height < 10 {
        return;
    }
    RatBlock::bordered()
        .border_type(RatBorder::Rounded)
        .style(rat_style(styles.card))
        .border_style(rat_style(styles.panel_border))
        .render(rat_rect(panel), buffer);
    let head = RatRect::new(panel.x + 2, panel.y + 1, panel.width - 4, 1);
    RatParagraph::new("Activity feed")
        .style(rat_style(styles.accent))
        .render(head, buffer);
    let builder = ListBuilder::new(|context| {
        (
            Item {
                index: context.index,
                picked: context.is_selected,
                styles: &styles,
            },
            height(context.index),
        )
    });
    let list = ListView::new(builder, ITEMS).style(rat_style(styles.card));
    let mut state = ListState::default();
    state.select(Some(selected(tick)));
    ratatui::widgets::StatefulWidget::render(list, rat_rect(list_area(panel)), buffer, &mut state);
}

/// Draws the fair-suite baseline with a reused dark theme, matching the native runner.
pub fn baseline(buffer: &mut Buffer, area: RatRect, tick: u32) {
    static THEME: OnceLock<Theme> = OnceLock::new();
    baseline_themed(
        buffer,
        area,
        THEME.get_or_init(|| Theme::builtin(Variant::Dark)),
        tick,
    );
}
