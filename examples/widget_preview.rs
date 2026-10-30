// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant};

use lunatui_bench::widget_suite::{CASES, Case, pin_color_output};
use lunatui_core::{
    Attrs, ClusterTable, Color, Grid, Rect, Style, UnderlineStyle, clip_display_prefix,
    width as cols,
};
use lunatui_runtime::keymap::{Command, KeyChord, Keymap};
use lunatui_term::{ColorMode, Encoder, KeyCode, Modifiers};
use lunatui_theme::{Profile, Theme, Token, Variant};
use lunatui_widgets::{
    Accordion, AccordionSection, Alignment, Axis, BigText, Block, BorderType, Breadcrumbs, Chart,
    ComboBox, Constraint, Dataset, Date, DatePicker, DateStyle, Diff, DiffLayout, DiffView,
    DividerId, FieldArea, Form, FormField, Gauge, GraphType, HexViewer, HexViewerState, KeyHintBar,
    List, LogBuffer, LogEntry, LogLevel, LogView, MarkdownView, Marker, Menu, MenuItem, MenuState,
    Monthly, NotificationCenter, NotificationId, NotificationLog, NumberInput, PaneId, Paragraph,
    PieChart, PieSlice, PixelSize, ProgressBar, ProgressTone, RadioFlow, RadioGroup, RenderCx,
    Scrollbar, SearchBox, SearchMatch, Slider, Sparkline, SplitAxis, SplitDrag, SplitTree,
    StatusBar, TabMove, Table, Tabs, TabsDrag, TagInput, TextInput, ToastKind, ToastScope,
    ToastStack, Toggle, VecSource, Widget, WidgetState,
};

mod bar_scene;
#[path = "button_showcase.rs"]
mod button_showcase;
#[path = "canvas_showcase.rs"]
mod canvas_showcase;
use bar_scene::BarBoard;
mod chart_scene;
mod spark_scene;
use chart_scene::ChartBoard;
#[path = "code_view_showcase.rs"]
mod code_view_showcase;
#[path = "deploy_board.rs"]
mod deploy_board;
#[path = "dialog_showcase.rs"]
mod dialog_showcase;
#[path = "menu_showcase.rs"]
mod menu_showcase;
#[path = "popover_showcase.rs"]
mod popover_showcase;
#[path = "release_console.rs"]
mod release_console;
#[path = "table_showcase.rs"]
mod table_showcase;
#[path = "tooltip_showcase.rs"]
mod tooltip_showcase;
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect as RatRect;
use ratatui::style::Color as RatColor;
use ratatui::style::Style as RatStyle;
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::{
    Block as RatBlock, BorderType as RatBorderType, Paragraph as RatParagraph, Widget as RatWidget,
};
use ratatui::{Terminal, TerminalOptions, Viewport};
use time::{Date as TimeDate, Month};
use tui_menu::{Menu as RatMenu, MenuItem as RatMenuItem, MenuState as RatMenuState};

type NativeDraw = fn(&mut Grid, &mut ClusterTable, &Theme, Rect, u32);
type BaselineDraw = fn(&mut Buffer, RatRect, &Theme, u32);

struct Preview {
    name: &'static str,
    native: NativeDraw,
    baseline: BaselineDraw,
}

const PREVIEWS: &[Preview] = &[
    Preview {
        name: "toast",
        native: toast_native,
        baseline: toast_baseline,
    },
    Preview {
        name: "split",
        native: split_native,
        baseline: split_baseline,
    },
    Preview {
        name: "tabs-drag",
        native: tabs_drag_native,
        baseline: tabs_drag_baseline,
    },
    Preview {
        name: "breadcrumbs",
        native: breadcrumbs_native,
        baseline: breadcrumbs_baseline,
    },
    Preview {
        name: "key-hints",
        native: key_hints_native,
        baseline: key_hints_baseline,
    },
    Preview {
        name: "list-icons",
        native: list_icons_native,
        baseline: list_icons_baseline,
    },
    Preview {
        name: "toggle",
        native: toggle_native,
        baseline: toggle_baseline,
    },
    Preview {
        name: "tooltip",
        native: tooltip_showcase::render,
        baseline: tooltip_baseline,
    },
    Preview {
        name: "popover",
        native: popover_showcase::render,
        baseline: popover_baseline,
    },
    Preview {
        name: "code-view",
        native: code_view_showcase::render,
        baseline: code_view_baseline,
    },
    Preview {
        name: "button",
        native: button_showcase::render,
        baseline: button_baseline,
    },
    Preview {
        name: "table-sorted",
        native: table_showcase::render,
        baseline: table_sorted_baseline,
    },
    Preview {
        name: "dialog",
        native: dialog_showcase::render,
        baseline: dialog_baseline,
    },
    Preview {
        name: "radio-group",
        native: radio_native,
        baseline: radio_baseline,
    },
    Preview {
        name: "form",
        native: form_native,
        baseline: form_baseline,
    },
    Preview {
        name: "progress-bar",
        native: progress_native,
        baseline: progress_baseline,
    },
    Preview {
        name: "accordion",
        native: accordion_native,
        baseline: accordion_baseline,
    },
    Preview {
        name: "hex-viewer",
        native: hex_viewer_native,
        baseline: hex_viewer_baseline,
    },
    Preview {
        name: "pie-chart",
        native: pie_native,
        baseline: pie_baseline,
    },
    Preview {
        name: "slider",
        native: slider_native,
        baseline: slider_baseline,
    },
    Preview {
        name: "number-input",
        native: number_native,
        baseline: number_baseline,
    },
    Preview {
        name: "search-box",
        native: search_native,
        baseline: search_baseline,
    },
    Preview {
        name: "date-picker",
        native: datepick_native,
        baseline: datepick_baseline,
    },
    Preview {
        name: "tag-input",
        native: tags_native,
        baseline: tags_baseline,
    },
    Preview {
        name: "notification-center",
        native: notify_native,
        baseline: notify_baseline,
    },
    Preview {
        name: "combo-box",
        native: combo_native,
        baseline: combo_baseline,
    },
    Preview {
        name: "logs",
        native: logs_native,
        baseline: logs_baseline,
    },
    Preview {
        name: "diff",
        native: diff_native,
        baseline: diff_baseline,
    },
    Preview {
        name: "calendar",
        native: calendar_native,
        baseline: calendar_baseline,
    },
    Preview {
        name: "bigtext",
        native: bigtext_native,
        baseline: bigtext_baseline,
    },
    Preview {
        name: "chart",
        native: chart_native,
        baseline: chart_baseline,
    },
    Preview {
        name: "barchart",
        native: barchart_native,
        baseline: barchart_baseline,
    },
    Preview {
        name: "markdown",
        native: markdown_native,
        baseline: markdown_baseline,
    },
    Preview {
        name: "canvas",
        native: canvas_native,
        baseline: canvas_baseline,
    },
    Preview {
        name: "sparkline",
        native: sparkline_native,
        baseline: sparkline_baseline,
    },
    Preview {
        name: "deploy",
        native: deploy_native,
        baseline: deploy_baseline,
    },
    Preview {
        name: "nested-menu",
        native: nested_menu_native,
        baseline: nested_menu_baseline,
    },
    Preview {
        name: "popup-scrollview",
        native: release_console::draw,
        baseline: release_console::baseline,
    },
];

fn preview(name: &str) -> Option<&'static Preview> {
    PREVIEWS.iter().find(|preview| preview.name == name)
}

fn native_preview(name: &str) -> Option<NativeDraw> {
    if name == "menu-editor" {
        Some(menu_showcase::draw)
    } else {
        preview(name).map(|preview| preview.native)
    }
}

fn preview_menu_state(tick: u32) -> MenuState<&'static str> {
    let mut state = MenuState::new(vec![
        MenuItem::group(
            "File",
            vec![
                MenuItem::item("New", "new"),
                MenuItem::item("Open", "open"),
                MenuItem::item("Save as", "save-as"),
                MenuItem::group(
                    "Recent",
                    vec![
                        MenuItem::item("file_1.txt", "recent-1"),
                        MenuItem::item("file_2.txt", "recent-2"),
                    ],
                ),
            ],
        ),
        MenuItem::group(
            "Edit",
            vec![
                MenuItem::item("Copy", "copy"),
                MenuItem::item("Cut", "cut"),
                MenuItem::item("Paste", "paste"),
            ],
        ),
        MenuItem::group(
            "About",
            vec![
                MenuItem::item("Author", "author"),
                MenuItem::item("Help", "help"),
            ],
        ),
    ]);
    let phase = (tick / 60 + 2) % 3;
    if phase > 0 {
        state.activate();
    }
    if phase > 1 {
        state.down();
        state.down();
        state.down();
        state.down();
        state.right();
    }
    state
}

fn preview_rat_menu_state(tick: u32) -> RatMenuState<&'static str> {
    let mut state = RatMenuState::new(vec![
        RatMenuItem::group(
            "File",
            vec![
                RatMenuItem::item("New", "new"),
                RatMenuItem::item("Open", "open"),
                RatMenuItem::item("Save as", "save-as"),
                RatMenuItem::group(
                    "Recent",
                    vec![
                        RatMenuItem::item("file_1.txt", "recent-1"),
                        RatMenuItem::item("file_2.txt", "recent-2"),
                    ],
                ),
            ],
        ),
        RatMenuItem::group(
            "Edit",
            vec![
                RatMenuItem::item("Copy", "copy"),
                RatMenuItem::item("Cut", "cut"),
                RatMenuItem::item("Paste", "paste"),
            ],
        ),
        RatMenuItem::group(
            "About",
            vec![
                RatMenuItem::item("Author", "author"),
                RatMenuItem::item("Help", "help"),
            ],
        ),
    ]);
    let phase = (tick / 60 + 2) % 3;
    if phase > 0 {
        state.activate();
    }
    if phase > 1 {
        state.down();
        state.down();
        state.down();
        state.down();
        state.right();
    }
    state
}

fn nested_menu_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    let text = Style::new()
        .fg(cx.token(Token::TextPrimary))
        .bg(cx.token(Token::Surface0));
    let body = Rect::new(
        area.x,
        area.y.saturating_add(1),
        area.width,
        area.height.saturating_sub(1),
    );
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title("Workspace")
        .style(text)
        .border_style(text.fg(cx.token(Token::TextMuted)))
        .title_style(text.fg(cx.token(Token::Accent)))
        .title_alignment(lunatui_widgets::Alignment::Right);
    let inner = block.inner(body);
    block.render(body, &mut cx);
    let content_x = inner.x.saturating_add(2);
    let content_width = inner.width.saturating_sub(4);
    Paragraph::new("Project: lunatui").style(text).render(
        Rect::new(content_x, inner.y.saturating_add(8), content_width, 1),
        &mut cx,
    );
    Paragraph::new("Changes staged: 4 files")
        .style(text)
        .render(
            Rect::new(content_x, inner.y.saturating_add(10), content_width, 1),
            &mut cx,
        );
    Paragraph::new("Ready to commit")
        .style(
            Style::new()
                .fg(cx.token(Token::Ok))
                .bg(cx.token(Token::Surface0)),
        )
        .render(
            Rect::new(content_x, inner.y.saturating_add(12), content_width, 1),
            &mut cx,
        );
    Menu::new(&preview_menu_state(tick))
        .state(WidgetState::Focused)
        .render(area, &mut cx);
}

fn nested_menu_baseline(buffer: &mut Buffer, area: RatRect, theme: &Theme, tick: u32) {
    let text = RatStyle::new()
        .fg(rat_color(theme.resolve(Token::TextPrimary)))
        .bg(rat_color(theme.resolve(Token::Surface0)));
    let body = RatRect::new(
        area.x,
        area.y.saturating_add(1),
        area.width,
        area.height.saturating_sub(1),
    );
    let block = RatBlock::bordered()
        .border_type(RatBorderType::Rounded)
        .title("Workspace")
        .style(text)
        .border_style(text.fg(rat_color(theme.resolve(Token::TextMuted))))
        .title_style(text.fg(rat_color(theme.resolve(Token::Accent))))
        .title_alignment(ratatui::layout::Alignment::Right);
    let inner = block.inner(body);
    RatWidget::render(block, body, buffer);
    let content_x = inner.x.saturating_add(2);
    let content_width = inner.width.saturating_sub(4);
    RatWidget::render(
        RatParagraph::new("Project: lunatui").style(text),
        RatRect::new(content_x, inner.y.saturating_add(8), content_width, 1),
        buffer,
    );
    RatWidget::render(
        RatParagraph::new("Changes staged: 4 files").style(text),
        RatRect::new(content_x, inner.y.saturating_add(10), content_width, 1),
        buffer,
    );
    RatWidget::render(
        RatParagraph::new("Ready to commit").style(
            RatStyle::new()
                .fg(rat_color(theme.resolve(Token::Ok)))
                .bg(rat_color(theme.resolve(Token::Surface0))),
        ),
        RatRect::new(content_x, inner.y.saturating_add(12), content_width, 1),
        buffer,
    );
    let selected = RatStyle::new()
        .fg(rat_color(theme.resolve(Token::TextMutedOnFocus)))
        .bg(rat_color(theme.resolve(Token::FocusFill)));
    StatefulWidget::render(
        RatMenu::new()
            .default_style(
                RatStyle::new()
                    .fg(rat_color(theme.resolve(Token::TextPrimary)))
                    .bg(rat_color(theme.resolve(Token::SurfaceRaised))),
            )
            .highlight(selected)
            .dropdown_width(20),
        area,
        buffer,
        &mut preview_rat_menu_state(tick),
    );
}

const DAY_HOLD: u32 = 11;
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const SHORT_DAYS: [&str; 7] = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];

#[derive(Clone, Copy)]
enum Kind {
    Release,
    Review,
    OnCall,
    Incident,
}

struct Entry {
    month: u8,
    day: u8,
    time: &'static str,
    title: &'static str,
    detail: &'static str,
    kind: Kind,
}

const ENTRIES: &[Entry] = &[
    Entry {
        month: 10,
        day: 22,
        time: "09:30",
        title: "Branch cut",
        detail: "Open the release branch for the preview.",
        kind: Kind::Release,
    },
    Entry {
        month: 10,
        day: 23,
        time: "15:00",
        title: "Design review",
        detail: "Walk the month and the agenda together.",
        kind: Kind::Review,
    },
    Entry {
        month: 10,
        day: 24,
        time: "11:00",
        title: "Docs pass",
        detail: "Read the gallery notes before they freeze.",
        kind: Kind::Review,
    },
    Entry {
        month: 10,
        day: 26,
        time: "10:00",
        title: "Ship window",
        detail: "Tag the preview and publish the notes.",
        kind: Kind::Release,
    },
    Entry {
        month: 10,
        day: 26,
        time: "16:30",
        title: "On-call handoff",
        detail: "Primary swaps at the end of the day.",
        kind: Kind::OnCall,
    },
    Entry {
        month: 10,
        day: 28,
        time: "14:00",
        title: "Incident review",
        detail: "Trace last week's error budget.",
        kind: Kind::Incident,
    },
    Entry {
        month: 10,
        day: 30,
        time: "17:00",
        title: "Docs freeze",
        detail: "Gallery and changelog stop moving.",
        kind: Kind::Review,
    },
    Entry {
        month: 11,
        day: 1,
        time: "18:00",
        title: "November board",
        detail: "Move open notes onto the new month.",
        kind: Kind::Review,
    },
    Entry {
        month: 11,
        day: 2,
        time: "10:00",
        title: "Sprint plan",
        detail: "Pick the next set of widgets.",
        kind: Kind::Review,
    },
    Entry {
        month: 11,
        day: 3,
        time: "09:00",
        title: "Beta cut",
        detail: "Cut the beta from the release branch.",
        kind: Kind::Release,
    },
    Entry {
        month: 11,
        day: 4,
        time: "13:00",
        title: "Demo",
        detail: "Show the schedule on a real screen.",
        kind: Kind::Release,
    },
    Entry {
        month: 11,
        day: 6,
        time: "15:00",
        title: "Retro",
        detail: "Walk what slowed the last cycle.",
        kind: Kind::Review,
    },
    Entry {
        month: 11,
        day: 9,
        time: "11:00",
        title: "On-call drill",
        detail: "Page the backup and time the reply.",
        kind: Kind::OnCall,
    },
];

#[derive(Clone, Copy)]
struct Ink {
    text: Color,
    surface: Color,
    raised: Color,
    accent: Color,
    ok: Color,
    warn: Color,
    fail: Color,
    focus: Color,
}

impl Ink {
    fn new(cx: &RenderCx<'_>) -> Self {
        Self {
            text: cx.token(Token::TextPrimary),
            surface: cx.token(Token::Surface0),
            raised: cx.token(Token::Surface1),
            accent: cx.token(Token::Accent),
            ok: cx.token(Token::Ok),
            warn: cx.token(Token::Warn),
            fail: cx.token(Token::Fail),
            focus: cx.token(Token::FocusRing),
        }
    }

    fn kind(self, kind: Kind) -> Color {
        match kind {
            Kind::Release => self.ok,
            Kind::Review => self.warn,
            Kind::OnCall => self.accent,
            Kind::Incident => self.fail,
        }
    }
}

struct Scene {
    ink: Ink,
    tick: u32,
    width: u16,
    height: u16,
    selected: Date,
    today: Date,
}

struct DayMarks {
    today: Date,
    selected: Date,
    raised: Color,
    release: Color,
    review: Color,
    oncall: Color,
    incident: Color,
}

impl DateStyle for DayMarks {
    fn style_for(&self, date: Date) -> Style {
        if date == self.selected {
            return Style::new();
        }
        let mut style = Style::new();
        if date == self.today {
            style = style.bg(self.raised);
        }
        if let Some(entry) = first_entry(date) {
            let color = match entry.kind {
                Kind::Release => self.release,
                Kind::Review => self.review,
                Kind::OnCall => self.oncall,
                Kind::Incident => self.incident,
            };
            style = style.fg(color);
        }
        style
    }
}

fn on_day(entry: &Entry, date: Date) -> bool {
    entry.month == date.month() && entry.day == date.day()
}

fn first_entry(date: Date) -> Option<&'static Entry> {
    ENTRIES.iter().find(|entry| on_day(entry, date))
}

fn month_name(date: Date) -> &'static str {
    MONTHS
        .get(usize::from(date.month().saturating_sub(1)))
        .copied()
        .unwrap_or("")
}

fn month_abbr(date: Date) -> &'static str {
    let name = month_name(date);
    name.get(..3).unwrap_or(name)
}

fn weekday_index(date: Date) -> usize {
    let Ok(month) = time::Month::try_from(date.month()) else {
        return 0;
    };
    let Ok(civil) = time::Date::from_calendar_date(date.year(), month, date.day()) else {
        return 0;
    };
    match civil.weekday() {
        time::Weekday::Sunday => 0,
        time::Weekday::Monday => 1,
        time::Weekday::Tuesday => 2,
        time::Weekday::Wednesday => 3,
        time::Weekday::Thursday => 4,
        time::Weekday::Friday => 5,
        time::Weekday::Saturday => 6,
    }
}

fn weekday_name(date: Date) -> &'static str {
    WEEKDAYS.get(weekday_index(date)).copied().unwrap_or("Day")
}

fn short_day(date: Date) -> &'static str {
    SHORT_DAYS.get(weekday_index(date)).copied().unwrap_or("Da")
}

fn add_days(date: Date, days: i32) -> Date {
    let Ok(month) = time::Month::try_from(date.month()) else {
        return date;
    };
    let Ok(civil) = time::Date::from_calendar_date(date.year(), month, date.day()) else {
        return date;
    };
    let Some(next) = civil.checked_add(time::Duration::days(i64::from(days))) else {
        return date;
    };
    Date::from_ymd(next.year(), u8::from(next.month()), next.day()).unwrap_or(date)
}

fn selected_on(tick: u32) -> Date {
    let start = Date::from_ymd(2026, 10, 22).unwrap_or(Date::UNIX_EPOCH);
    let steps = i32::try_from(tick / DAY_HOLD).unwrap_or(0);
    add_days(start, steps)
}

fn today_date() -> Date {
    Date::from_ymd(2026, 10, 26).unwrap_or(Date::UNIX_EPOCH)
}

fn bold(style: Style) -> Style {
    style.with_attrs(Attrs::new().set_bold(true))
}

fn paint(cx: &mut RenderCx<'_>, x: u16, y: u16, width: u16, text: &str, style: Style) {
    if width == 0 {
        return;
    }
    cx.put_str(x, y, clip_display_prefix(text, width), style);
}

fn hint_for(tick: u32, selected: Date) -> &'static str {
    let into = tick % DAY_HOLD;
    if tick > 0 && into < 8 && selected.day() == 1 {
        "n  month"
    } else if tick > 0 && into < 6 {
        "l  day"
    } else {
        "h l day     n month"
    }
}

fn draw_chrome(cx: &mut RenderCx<'_>, scene: &Scene) {
    let bar = Style::new().fg(scene.ink.text).bg(scene.ink.raised);
    cx.set_style(Rect::new(0, 0, scene.width, 1), bar);
    let badge = bold(Style::new().fg(scene.ink.surface).bg(scene.ink.ok));
    paint(cx, 1, 0, 6, " live ", badge);
    paint(cx, 8, 0, 12, "schedule", bold(bar));
    let month = format!("{} {}", month_name(scene.selected), scene.selected.year());
    let month_cols = u16::try_from(month.len()).unwrap_or(0);
    let month_x = scene.width.saturating_sub(month_cols) / 2;
    if month_x > 20 {
        paint(
            cx,
            month_x,
            0,
            month_cols,
            &month,
            bold(Style::new().fg(scene.ink.accent).bg(scene.ink.raised)),
        );
    }
    let second = (scene.tick / 30) % 60;
    let clock = format!("14:02:{second:02}");
    let clock_cols = u16::try_from(clock.len()).unwrap_or(0);
    let clock_x = scene.width.saturating_sub(clock_cols.saturating_add(1));
    paint(
        cx,
        clock_x,
        0,
        clock_cols,
        &clock,
        Style::new().fg(scene.ink.accent).bg(scene.ink.raised),
    );
    paint(
        cx,
        1,
        1,
        28,
        "prod     team calendar",
        Style::new().fg(scene.ink.text).bg(scene.ink.surface),
    );
    let footer_y = scene.height.saturating_sub(1);
    cx.set_style(Rect::new(0, footer_y, scene.width, 1), bar);
    let hint = hint_for(scene.tick, scene.selected);
    let hint_color = if hint.starts_with('n') {
        scene.ink.accent
    } else if hint.starts_with('l') {
        scene.ink.focus
    } else {
        scene.ink.text
    };
    paint(
        cx,
        1,
        footer_y,
        24,
        hint,
        Style::new().fg(hint_color).bg(scene.ink.raised),
    );
    let stamp = format!(
        "{:04}-{:02}-{:02}",
        scene.selected.year(),
        scene.selected.month(),
        scene.selected.day()
    );
    let stamp_cols = u16::try_from(stamp.len()).unwrap_or(0);
    paint(
        cx,
        scene.width.saturating_sub(stamp_cols.saturating_add(1)),
        footer_y,
        stamp_cols,
        &stamp,
        Style::new().fg(scene.ink.focus).bg(scene.ink.raised),
    );
}

fn day_marks(scene: &Scene) -> DayMarks {
    DayMarks {
        today: scene.today,
        selected: scene.selected,
        raised: scene.ink.raised,
        release: scene.ink.ok,
        review: scene.ink.warn,
        oncall: scene.ink.accent,
        incident: scene.ink.fail,
    }
}

fn draw_month(cx: &mut RenderCx<'_>, scene: &Scene, area: Rect) {
    if area.width < 23 || area.height < 9 {
        return;
    }
    let ground = Style::new().fg(scene.ink.text).bg(scene.ink.surface);
    let frame = Block::bordered()
        .border_type(BorderType::Rounded)
        .style(ground)
        .border_style(Style::new().fg(scene.ink.focus).bg(scene.ink.surface));
    frame.render(area, cx);
    let inner = frame.inner(area);
    let cal_w = 21_u16;
    if inner.width < cal_w || inner.height < 7 {
        return;
    }
    let pad = inner.width.saturating_sub(cal_w) / 2;
    Monthly::new(scene.selected, day_marks(scene))
        .state(WidgetState::Focused)
        .render(
            Rect::new(
                inner.x.saturating_add(pad),
                inner.y,
                cal_w,
                inner.height.min(8),
            ),
            cx,
        );
}

fn draw_legend(cx: &mut RenderCx<'_>, scene: &Scene, area: Rect) {
    if area.height == 0 || area.width < 12 {
        return;
    }
    let y = area.y;
    paint(
        cx,
        area.x,
        y,
        12,
        "• release",
        Style::new().fg(scene.ink.ok).bg(scene.ink.surface),
    );
    paint(
        cx,
        area.x.saturating_add(14),
        y,
        10,
        "• review",
        Style::new().fg(scene.ink.warn).bg(scene.ink.surface),
    );
    if area.height > 1 {
        let next = y.saturating_add(1);
        paint(
            cx,
            area.x,
            next,
            12,
            "• on-call",
            Style::new().fg(scene.ink.accent).bg(scene.ink.surface),
        );
        paint(
            cx,
            area.x.saturating_add(14),
            next,
            12,
            "• incident",
            Style::new().fg(scene.ink.fail).bg(scene.ink.surface),
        );
    }
}

fn week_line(date: Date) -> String {
    let count = ENTRIES.iter().filter(|entry| on_day(entry, date)).count();
    match first_entry(date) {
        Some(entry) if count > 1 => format!(
            "{} {:02}  {} +{}",
            short_day(date),
            date.day(),
            entry.title,
            count - 1
        ),
        Some(entry) => format!("{} {:02}  {}", short_day(date), date.day(), entry.title),
        None => format!("{} {:02}", short_day(date), date.day()),
    }
}

fn paint_day_row(cx: &mut RenderCx<'_>, scene: &Scene, x: u16, y: u16, width: u16, date: Date) {
    let picked = date == scene.selected;
    let text = Style::new().fg(scene.ink.text).bg(scene.ink.surface);
    let style = if picked {
        Style::new().fg(scene.ink.focus).bg(scene.ink.raised)
    } else if date == scene.today {
        Style::new().fg(scene.ink.ok).bg(scene.ink.surface)
    } else {
        text
    };
    if picked {
        cx.set_style(
            Rect::new(x, y, width, 1),
            Style::new().fg(scene.ink.text).bg(scene.ink.raised),
        );
    }
    paint(
        cx,
        x,
        y,
        width,
        &week_line(date),
        if picked { bold(style) } else { style },
    );
}

fn draw_week(cx: &mut RenderCx<'_>, scene: &Scene, area: Rect) {
    if area.height < 4 || area.width < 12 {
        return;
    }
    let label = Style::new().fg(scene.ink.accent).bg(scene.ink.surface);
    paint(cx, area.x, area.y, area.width, "This week", bold(label));
    let index = weekday_index(scene.selected);
    let sunday = add_days(scene.selected, -i32::try_from(index).unwrap_or(0));
    let bottom = area.y.saturating_add(area.height);
    let mut row = area.y.saturating_add(1);
    let mut offset = 0_i32;
    let mut labeled_next = false;
    while row < bottom {
        if offset >= 7 && !labeled_next {
            if row.saturating_add(1) >= bottom {
                break;
            }
            paint(cx, area.x, row, area.width, "Next", bold(label));
            labeled_next = true;
            row = row.saturating_add(1);
            continue;
        }
        paint_day_row(cx, scene, area.x, row, area.width, add_days(sunday, offset));
        offset = offset.saturating_add(1);
        row = row.saturating_add(1);
    }
}

fn draw_event_chip(cx: &mut RenderCx<'_>, scene: &Scene, area: Rect, entry: &Entry, y: u16) {
    let chip = Style::new().fg(scene.ink.text).bg(scene.ink.raised);
    cx.set_style(Rect::new(area.x, y, area.width, 2), chip);
    let mark = Style::new()
        .fg(scene.ink.kind(entry.kind))
        .bg(scene.ink.raised);
    paint(cx, area.x, y, 1, "▌", mark);
    paint(cx, area.x.saturating_add(2), y, 5, entry.time, mark);
    paint(
        cx,
        area.x.saturating_add(9),
        y,
        area.width.saturating_sub(9),
        entry.title,
        bold(chip),
    );
    paint(
        cx,
        area.x.saturating_add(2),
        y.saturating_add(1),
        area.width.saturating_sub(2),
        entry.detail,
        chip,
    );
}

fn upcoming_count(selected: Date) -> u16 {
    let mut count = 0_u16;
    for step in 1..22 {
        let date = add_days(selected, step);
        for _entry in ENTRIES.iter().filter(|entry| on_day(entry, date)) {
            count = count.saturating_add(1);
        }
    }
    count
}

fn draw_month_ledger(cx: &mut RenderCx<'_>, scene: &Scene, area: Rect, y: u16) {
    let bottom = area.y.saturating_add(area.height);
    if y.saturating_add(1) >= bottom {
        return;
    }
    paint(
        cx,
        area.x,
        y,
        area.width,
        month_name(scene.selected),
        bold(Style::new().fg(scene.ink.accent).bg(scene.ink.surface)),
    );
    let start =
        Date::from_ymd(scene.selected.year(), scene.selected.month(), 1).unwrap_or(scene.selected);
    let mut line = y.saturating_add(1);
    let mut offset = 0_i32;
    while line < bottom {
        paint_day_row(cx, scene, area.x, line, area.width, add_days(start, offset));
        offset = offset.saturating_add(1);
        line = line.saturating_add(1);
    }
}

fn draw_up_next(cx: &mut RenderCx<'_>, scene: &Scene, area: Rect, y: u16) -> u16 {
    if upcoming_count(scene.selected) == 0
        || y.saturating_add(1) >= area.y.saturating_add(area.height)
    {
        return y;
    }
    paint(
        cx,
        area.x,
        y,
        area.width,
        "Up next",
        bold(Style::new().fg(scene.ink.accent).bg(scene.ink.surface)),
    );
    let mut line = y.saturating_add(1);
    let bottom = area.y.saturating_add(area.height);
    let text = Style::new().fg(scene.ink.text).bg(scene.ink.surface);
    for step in 1..22 {
        if line >= bottom {
            break;
        }
        let date = add_days(scene.selected, step);
        for entry in ENTRIES.iter().filter(|entry| on_day(entry, date)) {
            if line >= bottom {
                break;
            }
            paint(
                cx,
                area.x,
                line,
                area.width,
                &format!(
                    "{:02} {}  {}  {}",
                    date.day(),
                    month_abbr(date),
                    entry.time,
                    entry.title
                ),
                text.fg(scene.ink.kind(entry.kind)),
            );
            line = line.saturating_add(1);
        }
    }
    line
}

fn draw_agenda(cx: &mut RenderCx<'_>, scene: &Scene, area: Rect) {
    if area.width < 24 || area.height < 8 {
        return;
    }
    let ground = Style::new().fg(scene.ink.text).bg(scene.ink.surface);
    let title = if scene.selected == scene.today {
        format!(
            " {} {} {}  ·  today ",
            weekday_name(scene.selected),
            scene.selected.day(),
            month_name(scene.selected)
        )
    } else {
        format!(
            " {} {} {} ",
            weekday_name(scene.selected),
            scene.selected.day(),
            month_name(scene.selected)
        )
    };
    let frame = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(title)
        .style(ground)
        .border_style(Style::new().fg(scene.ink.accent).bg(scene.ink.surface))
        .title_style(bold(Style::new().fg(scene.ink.text).bg(scene.ink.surface)));
    frame.render(area, cx);
    let inner = frame.inner(area);
    if inner.width < 16 || inner.height < 4 {
        return;
    }
    let count = ENTRIES
        .iter()
        .filter(|entry| on_day(entry, scene.selected))
        .count();
    let summary = if count == 0 {
        "Clear day".to_owned()
    } else if count == 1 {
        "1 scheduled".to_owned()
    } else {
        format!("{count} scheduled")
    };
    paint(
        cx,
        inner.x,
        inner.y,
        inner.width,
        &summary,
        Style::new().fg(scene.ink.focus).bg(scene.ink.surface),
    );
    let mut y = inner.y.saturating_add(2);
    let bottom = inner.y.saturating_add(inner.height);
    for entry in ENTRIES.iter().filter(|entry| on_day(entry, scene.selected)) {
        if y.saturating_add(2) >= bottom {
            break;
        }
        draw_event_chip(cx, scene, inner, entry, y);
        y = y.saturating_add(3);
    }
    if count == 0 && y < bottom {
        paint(cx, inner.x, y, inner.width, "Nothing scheduled.", ground);
        y = y.saturating_add(2);
    } else {
        y = y.saturating_add(1);
    }
    let next = draw_up_next(cx, scene, inner, y);
    draw_month_ledger(cx, scene, inner, next);
}

fn draw_board(cx: &mut RenderCx<'_>, scene: &Scene) {
    let top = 3_u16;
    let bottom = scene.height.saturating_sub(2);
    if bottom <= top {
        return;
    }
    let left_w = if scene.width >= 110 { 40 } else { 32 };
    let month = Rect::new(1, top, left_w, 9);
    draw_month(cx, scene, month);
    let mut y = top.saturating_add(10);
    if bottom.saturating_sub(y) >= 10 {
        draw_legend(cx, scene, Rect::new(2, y, left_w.saturating_sub(2), 2));
        y = y.saturating_add(3);
    }
    draw_week(
        cx,
        scene,
        Rect::new(2, y, left_w.saturating_sub(2), bottom.saturating_sub(y)),
    );
    let agenda_x = left_w.saturating_add(2);
    draw_agenda(
        cx,
        scene,
        Rect::new(
            agenda_x,
            top,
            scene.width.saturating_sub(agenda_x).saturating_sub(1),
            bottom.saturating_sub(top),
        ),
    );
}

fn calendar_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let text = theme.resolve(Token::TextPrimary);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let scene = Scene {
        ink: Ink::new(&cx),
        tick,
        width: area.width,
        height: area.height,
        selected: selected_on(tick),
        today: today_date(),
    };
    draw_chrome(&mut cx, &scene);
    if scene.width < 64 || scene.height < 20 {
        draw_month(
            &mut cx,
            &scene,
            Rect::new(
                1,
                3,
                scene.width.saturating_sub(2),
                scene.height.saturating_sub(5),
            ),
        );
        return;
    }
    draw_board(&mut cx, &scene);
}

struct NoEvents;

impl ratatui::widgets::calendar::DateStyler for NoEvents {
    fn get_style(&self, _date: TimeDate) -> ratatui::style::Style {
        ratatui::style::Style::new()
    }
}

fn calendar_baseline(buffer: &mut Buffer, area: RatRect, theme: &Theme, tick: u32) {
    let month = match ((tick / 4) % 12) + 1 {
        1 => Month::January,
        2 => Month::February,
        3 => Month::March,
        4 => Month::April,
        5 => Month::May,
        6 => Month::June,
        7 => Month::July,
        8 => Month::August,
        9 => Month::September,
        10 => Month::October,
        11 => Month::November,
        _ => Month::December,
    };
    let date = TimeDate::from_calendar_date(2026, month, 1).unwrap_or(TimeDate::MIN);
    let color = match theme.resolve(Token::Accent) {
        Color::Rgb(red, green, blue) => RatColor::Rgb(red, green, blue),
        Color::Indexed(index) => RatColor::Indexed(index),
        _ => RatColor::Reset,
    };
    let style = ratatui::style::Style::default().fg(color);
    ratatui::widgets::Widget::render(
        ratatui::widgets::calendar::Monthly::new(date, NoEvents)
            .default_style(style)
            .show_month_header(style)
            .show_weekdays_header(style)
            .show_surrounding(style),
        area,
        buffer,
    );
}

#[derive(Clone, Copy)]
struct CutInk {
    text: Color,
    accent: Color,
    surface: Color,
    raised: Color,
    focus: Color,
    ok: Color,
    warn: Color,
    fail: Color,
}

impl CutInk {
    fn new(cx: &RenderCx<'_>) -> Self {
        Self {
            text: cx.token(Token::TextPrimary),
            accent: cx.token(Token::Accent),
            surface: cx.token(Token::Surface0),
            raised: cx.token(Token::Surface1),
            focus: cx.token(Token::FocusRing),
            ok: cx.token(Token::Ok),
            warn: cx.token(Token::Warn),
            fail: cx.token(Token::Fail),
        }
    }
}

struct CutScene {
    ink: CutInk,
    tick: u32,
    width: u16,
    height: u16,
}

impl CutScene {
    fn second(&self) -> u32 {
        (self.tick / 30) % 60
    }

    fn remain(&self) -> u32 {
        6_u32.saturating_sub(self.tick / 30)
    }

    fn live(&self) -> bool {
        self.tick / 30 >= 6
    }

    fn metric(&self) -> usize {
        let tick = self.tick;
        if (72..120).contains(&tick) {
            1
        } else if (120..192).contains(&tick) {
            2
        } else {
            0
        }
    }
}

fn requests(tick: u32) -> u32 {
    1_480 + tick.saturating_mul(2)
}

fn latency(tick: u32) -> u32 {
    60 + (tick / 3) % 30
}

fn error_count(tick: u32) -> u32 {
    if tick < 120 {
        1
    } else if tick < 192 {
        2 + ((tick - 120) / 3).min(12)
    } else {
        14_u32.saturating_sub((tick - 192) / 2).max(3)
    }
}

fn live_count(tick: u32) -> u32 {
    18_420 + tick.saturating_mul(17)
}

fn count_text(tick: u32) -> String {
    let count = live_count(tick);
    format!("{count:06}")
}

fn cut_hint(tick: u32) -> &'static str {
    let into = tick % 48;
    if tick > 0 && into < 8 {
        "l  metric"
    } else {
        "h l metric"
    }
}

fn draw_cut_chrome(cx: &mut RenderCx<'_>, scene: &CutScene) {
    let bar = Style::new().fg(scene.ink.text).bg(scene.ink.raised);
    cx.set_style(Rect::new(0, 0, scene.width, 1), bar);
    paint(
        cx,
        1,
        0,
        6,
        " live ",
        bold(Style::new().fg(scene.ink.surface).bg(scene.ink.ok)),
    );
    paint(cx, 8, 0, 10, "cutover", bold(bar));
    let clock = format!("14:02:{:02}", scene.second());
    let clock_cols = u16::try_from(clock.len()).unwrap_or(0);
    paint(
        cx,
        scene.width.saturating_sub(clock_cols.saturating_add(1)),
        0,
        clock_cols,
        &clock,
        Style::new().fg(scene.ink.accent).bg(scene.ink.raised),
    );
    paint(
        cx,
        1,
        1,
        32,
        "prod     preview window",
        Style::new().fg(scene.ink.text).bg(scene.ink.surface),
    );
    let footer_y = scene.height.saturating_sub(1);
    cx.set_style(Rect::new(0, footer_y, scene.width, 1), bar);
    let hint = cut_hint(scene.tick);
    let hint_color = if hint.starts_with('l') {
        scene.ink.focus
    } else {
        scene.ink.text
    };
    paint(
        cx,
        1,
        footer_y,
        16,
        hint,
        Style::new().fg(hint_color).bg(scene.ink.raised),
    );
    let stamp = if scene.live() { "live" } else { "waiting" };
    let stamp_cols = u16::try_from(stamp.len()).unwrap_or(0);
    let stamp_color = if scene.live() {
        scene.ink.ok
    } else {
        scene.ink.warn
    };
    paint(
        cx,
        scene.width.saturating_sub(stamp_cols.saturating_add(1)),
        footer_y,
        stamp_cols,
        stamp,
        Style::new().fg(stamp_color).bg(scene.ink.raised),
    );
}

fn hero_size(width: u16) -> PixelSize {
    if width >= 100 {
        PixelSize::HalfHeight
    } else {
        PixelSize::Quadrant
    }
}

fn draw_clock(cx: &mut RenderCx<'_>, scene: &CutScene, y: u16) {
    let clock = format!("14:02:{:02}", scene.second());
    let size = hero_size(scene.width);
    let (_width, height) = size.cells();
    BigText::new(&clock)
        .pixel_size(size)
        .alignment(Alignment::Center)
        .state(WidgetState::Focused)
        .render(Rect::new(0, y, scene.width, height), cx);
}

fn draw_status(cx: &mut RenderCx<'_>, scene: &CutScene, y: u16) {
    let word = if scene.live() { "LIVE" } else { "WAIT" };
    let color = if scene.live() {
        scene.ink.ok
    } else {
        scene.ink.warn
    };
    let size = hero_size(scene.width);
    let (_width, height) = size.cells();
    BigText::new(word)
        .pixel_size(size)
        .alignment(Alignment::Center)
        .style(Style::new().fg(color))
        .render(Rect::new(0, y, scene.width, height), cx);
}

fn draw_countdown(cx: &mut RenderCx<'_>, scene: &CutScene, y: u16) {
    let left = format!("00:00:{:02}", scene.remain());
    let color = if scene.live() {
        scene.ink.ok
    } else {
        scene.ink.warn
    };
    let size = hero_size(scene.width);
    let (_width, height) = size.cells();
    BigText::new(&left)
        .pixel_size(size)
        .alignment(Alignment::Center)
        .style(Style::new().fg(color))
        .render(Rect::new(0, y, scene.width, height), cx);
}

fn draw_metric(
    cx: &mut RenderCx<'_>,
    scene: &CutScene,
    area: Rect,
    label: &str,
    value: &str,
    hot: bool,
    picked: bool,
) {
    let (_width, glyph_h) = PixelSize::HalfHeight.cells();
    if area.width < 8 || area.height < glyph_h.saturating_add(1) {
        return;
    }
    let label_style = if picked {
        bold(Style::new().fg(scene.ink.focus).bg(scene.ink.raised))
    } else {
        Style::new().fg(scene.ink.text).bg(scene.ink.surface)
    };
    if picked {
        cx.set_style(area, Style::new().fg(scene.ink.text).bg(scene.ink.raised));
    }
    let label_cols = u16::try_from(label.len()).unwrap_or(0);
    let label_x = area.x + area.width.saturating_sub(label_cols) / 2;
    paint(cx, label_x, area.y, label_cols, label, label_style);
    let mut text = BigText::new(value)
        .pixel_size(PixelSize::HalfHeight)
        .alignment(Alignment::Center);
    text = if hot {
        text.style(Style::new().fg(scene.ink.fail))
    } else if picked {
        text.state(WidgetState::Focused)
    } else {
        text.state(WidgetState::Normal)
    };
    text.render(
        Rect::new(area.x, area.y.saturating_add(1), area.width, glyph_h),
        cx,
    );
}

fn draw_metrics(cx: &mut RenderCx<'_>, scene: &CutScene, y: u16) {
    let gap = 2_u16;
    let col_w = scene
        .width
        .saturating_sub(4)
        .saturating_sub(gap.saturating_mul(2))
        / 3;
    let (glyph_w, _) = PixelSize::HalfHeight.cells();
    if col_w < glyph_w.saturating_mul(4) {
        return;
    }
    let values = [
        ("req/s", format!("{:04}", requests(scene.tick)), false),
        ("p99 ms", format!("{:03}", latency(scene.tick)), false),
        (
            "errors",
            format!("{:03}", error_count(scene.tick)),
            error_count(scene.tick) >= 8,
        ),
    ];
    let picked = scene.metric();
    for (index, (label, value, hot)) in values.iter().enumerate() {
        let x = 2 + u16::try_from(index).unwrap_or(0) * (col_w + gap);
        let (_width, glyph_h) = PixelSize::HalfHeight.cells();
        draw_metric(
            cx,
            scene,
            Rect::new(x, y, col_w, glyph_h.saturating_add(1)),
            label,
            value,
            *hot,
            index == picked,
        );
    }
}

fn readout(scene: &CutScene) -> String {
    match scene.metric() {
        0 => format!("requests  {} /s", requests(scene.tick)),
        1 => format!("p99  {} ms", latency(scene.tick)),
        _ => format!("errors  {}", error_count(scene.tick)),
    }
}

fn draw_cut_activity(cx: &mut RenderCx<'_>, scene: &CutScene, y: u16) {
    let height = scene.height.saturating_sub(y.saturating_add(1));
    if height < 5 {
        return;
    }
    let width = scene.width.saturating_sub(6) * 2 / 5;
    let plot = Rect::new(2, y, width, height);
    let events = Rect::new(width + 4, y, scene.width.saturating_sub(width + 6), height);
    let ground = Style::new().fg(scene.ink.text).bg(scene.ink.surface);
    let frame = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" REQUEST RATE ")
        .style(ground)
        .border_style(ground.fg(scene.ink.accent));
    frame.render(plot, cx);
    let inner = frame.inner(plot);
    let samples: Vec<u64> = (0..inner.width)
        .map(|column| {
            let age = u32::from(inner.width - column - 1) * 6;
            u64::from(requests(scene.tick.saturating_sub(age)))
        })
        .collect();
    Sparkline::new(&samples)
        .max(2_200)
        .style(ground.fg(scene.ink.ok))
        .render(
            Rect::new(inner.x, inner.y, inner.width, inner.height - 1),
            cx,
        );
    let min = samples.first().copied().unwrap_or(0);
    let label = format!(
        "{} req/s   min {min}   max {}",
        requests(scene.tick),
        requests(scene.tick)
    );
    Paragraph::new(&label).style(ground).render(
        Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1),
        cx,
    );

    let frame = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" CUTOVER EVENTS ")
        .style(ground)
        .border_style(ground.fg(scene.ink.accent));
    frame.render(events, cx);
    let mut log = LogBuffer::new(8);
    for (at, level, message) in [
        (0, LogLevel::Info, "Standby pool ready".to_owned()),
        (
            30,
            LogLevel::Info,
            format!("Warmup: {} req/s", requests(30)),
        ),
        (
            90,
            LogLevel::Info,
            format!("Readiness p99: {} ms", latency(90)),
        ),
        (
            150,
            LogLevel::Warn,
            format!("Retry queue: {} errors", error_count(150)),
        ),
        (
            180,
            LogLevel::Info,
            "Cutover complete; traffic live".to_owned(),
        ),
        (
            210,
            LogLevel::Info,
            format!("Recovery: {} errors", error_count(210)),
        ),
        (
            270,
            LogLevel::Info,
            format!("Serving {} req/s", requests(270)),
        ),
        (300, LogLevel::Info, "Standby pool drained".to_owned()),
    ] {
        if scene.tick >= at {
            log.push(LogEntry::new(level, "", message).stamp(format!("14:02:{:02}", at / 30)));
        }
    }
    LogView::new(&log)
        .target_width(0)
        .follow(true)
        .render(frame.inner(events), cx);
}

fn draw_cut_board(cx: &mut RenderCx<'_>, scene: &CutScene) {
    let clock_y = 2;
    draw_clock(cx, scene, clock_y);
    let (_width, clock_h) = hero_size(scene.width).cells();
    let caption_y = clock_y.saturating_add(clock_h);
    let caption = "local time";
    let caption_cols = u16::try_from(caption.len()).unwrap_or(0);
    paint(
        cx,
        scene.width.saturating_sub(caption_cols) / 2,
        caption_y,
        caption_cols,
        caption,
        Style::new().fg(scene.ink.text).bg(scene.ink.surface),
    );
    let status_y = caption_y.saturating_add(2);
    draw_status(cx, scene, status_y);
    let (_width, status_h) = hero_size(scene.width).cells();
    let label_y = status_y.saturating_add(status_h);
    let label = "cutover";
    let label_cols = u16::try_from(label.len()).unwrap_or(0);
    paint(
        cx,
        scene.width.saturating_sub(label_cols) / 2,
        label_y,
        label_cols,
        label,
        Style::new().fg(scene.ink.accent).bg(scene.ink.surface),
    );
    let count_y = label_y.saturating_add(1);
    draw_countdown(cx, scene, count_y);
    let (_width, count_h) = hero_size(scene.width).cells();
    let metrics_y = count_y.saturating_add(count_h).saturating_add(1);
    draw_metrics(cx, scene, metrics_y);
    let (_width, metric_h) = PixelSize::HalfHeight.cells();
    let note = readout(scene);
    paint(
        cx,
        2,
        metrics_y.saturating_add(metric_h).saturating_add(1),
        scene.width.saturating_sub(4),
        &note,
        Style::new().fg(scene.ink.focus).bg(scene.ink.surface),
    );
    draw_cut_activity(
        cx,
        scene,
        metrics_y.saturating_add(metric_h).saturating_add(2),
    );
}

fn draw_compact(cx: &mut RenderCx<'_>, scene: &CutScene) {
    let clock_y = 2;
    draw_clock(cx, scene, clock_y);
    let (_width, clock_h) = hero_size(scene.width).cells();
    let count_y = clock_y.saturating_add(clock_h).saturating_add(1);
    let (_width, count_h) = hero_size(scene.width).cells();
    if count_y.saturating_add(count_h) < scene.height.saturating_sub(1) {
        draw_countdown(cx, scene, count_y);
    }
}

fn bigtext_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let text = theme.resolve(Token::TextPrimary);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let scene = CutScene {
        ink: CutInk::new(&cx),
        tick,
        width: area.width,
        height: area.height,
    };
    draw_cut_chrome(&mut cx, &scene);
    if scene.width < 100 || scene.height < 34 {
        draw_compact(&mut cx, &scene);
        return;
    }
    draw_cut_board(&mut cx, &scene);
}

fn bigtext_baseline(buffer: &mut Buffer, area: RatRect, theme: &Theme, tick: u32) {
    let count = count_text(tick);
    let text = BigText::new(&count).pixel_size(PixelSize::Quadrant);
    let color = match theme.resolve(Token::Accent) {
        Color::Rgb(red, green, blue) => RatColor::Rgb(red, green, blue),
        Color::Indexed(index) => RatColor::Indexed(index),
        _ => RatColor::Reset,
    };
    let style = ratatui::style::Style::default().fg(color);
    let (_width, height) = PixelSize::Quadrant.cells();
    for row in 0..height.min(area.height) {
        let mut glyphs = [' '; 64];
        let columns = text.row_glyphs(row, &mut glyphs);
        let y = area.y.saturating_add(row);
        let limit = columns.min(glyphs.len()).min(usize::from(area.width));
        for (col, glyph) in glyphs.iter().take(limit).enumerate() {
            let x = area.x.saturating_add(u16::try_from(col).unwrap_or(0));
            if x >= area.x.saturating_add(area.width) {
                break;
            }
            buffer[(x, y)].set_char(*glyph).set_style(style);
        }
    }
}
fn wave(index: u32) -> u64 {
    let phase = index % 72;
    let rise = if phase < 36 { phase } else { 72 - phase };
    u64::from(18 + rise.saturating_mul(3) + (index % 11).saturating_mul(4))
}

fn fill_wave(samples: &mut [u64], tick: u32) {
    for (offset, sample) in samples.iter_mut().enumerate() {
        let index = tick.saturating_add(u32::try_from(offset).unwrap_or(0));
        *sample = wave(index);
    }
}

fn tri(t: u32, period: u32, low: u64, high: u64) -> u64 {
    let period = period.max(2);
    let phase = t % period;
    let half = period / 2;
    let up = if phase <= half { phase } else { period - phase };
    let span = high.saturating_sub(low);
    low.saturating_add(span.saturating_mul(u64::from(up)) / u64::from(half.max(1)))
}

fn sparkline_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let text = theme.resolve(Token::TextPrimary);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    spark_scene::render_scene(area, tick, false, &mut RenderCx::new(grid, clusters, theme));
}

fn deploy_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    _area: Rect,
    tick: u32,
) {
    deploy_board::draw(grid, clusters, theme, u64::from(tick).saturating_mul(100));
}

fn deploy_baseline(buffer: &mut Buffer, area: RatRect, theme: &Theme, tick: u32) {
    let accent = match theme.resolve(Token::Accent) {
        Color::Rgb(red, green, blue) => ratatui::style::Color::Rgb(red, green, blue),
        Color::Indexed(index) => ratatui::style::Color::Indexed(index),
        _ => ratatui::style::Color::Reset,
    };
    let block = ratatui::widgets::Block::default()
        .title(" production deploy ")
        .borders(ratatui::widgets::Borders::ALL);
    let inner = block.inner(area);
    ratatui::widgets::Widget::render(block, area, buffer);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    ratatui::widgets::Widget::render(
        ratatui::widgets::Paragraph::new("release 2.4.1    us-west-3    rolling"),
        RatRect::new(inner.x, inner.y, inner.width, 1),
        buffer,
    );
    let ratio = f64::from(tick % 80) / 80.0;
    let bar_y = inner.y.saturating_add(inner.height.saturating_sub(3));
    let bar_h = inner.height.min(3);
    ratatui::widgets::Widget::render(
        ratatui::widgets::Gauge::default()
            .ratio(ratio)
            .label("rollout")
            .gauge_style(ratatui::style::Style::default().fg(accent)),
        RatRect::new(inner.x, bar_y, inner.width, bar_h),
        buffer,
    );
}

fn sparkline_baseline(
    buffer: &mut ratatui::buffer::Buffer,
    area: RatRect,
    theme: &Theme,
    tick: u32,
) {
    let mut samples = [0; 160];
    let columns = usize::from(area.width).min(samples.len());
    fill_wave(&mut samples[..columns], tick.saturating_mul(2));
    let color = match theme.resolve(Token::Accent) {
        Color::Rgb(red, green, blue) => ratatui::style::Color::Rgb(red, green, blue),
        Color::Indexed(index) => ratatui::style::Color::Indexed(index),
        _ => ratatui::style::Color::Reset,
    };
    ratatui::widgets::Widget::render(
        ratatui::widgets::Sparkline::default()
            .data(samples[..columns].to_vec())
            .style(ratatui::style::Style::default().fg(color)),
        area,
        buffer,
    );
}

const MD_PAGES: [(&str, &str); 3] = [
    (
        "Getting started",
        "# Getting started\n\nLunatui draws terminal interfaces from small **widgets**. Each widget takes an area and a `RenderCx` and writes cells; nothing else is needed to get a frame on screen.\n\n## Install\n\n1. Add the crate with the features you use.\n2. Create a `Grid` and a `ClusterTable`.\n3. Render a widget into the grid.\n4. Diff the grid and write the changes.\n\n```toml\n[dependencies]\nlunatui = \"0.1\"\nlunatui-widgets = { version = \"0.1\", features = [\"markdown\"] }\n```\n\n## Your first frame\n\n> Every widget redraws the whole of its own area, so a frame never shows leftovers from the one before it.\n\n- [x] Block and Paragraph\n- [x] List and Table\n- [ ] Your own widget\n  - implement `Widget`\n  - keep `render` free of side effects\n\n## Themes\n\nColors come from semantic tokens, so a theme change recolors every widget without touching the widget code. The built-in dark and light themes cover the common terminals, and a prepared palette downsamples once instead of on every cell.\n\n| token | used for |\n|:--|:--|\n| `Accent` | headings and highlights |\n| `FocusRing` | links and focus |\n| `Surface1` | raised panels and code |\n\n## Input\n\nKeys, mouse, and paste events arrive as plain values. The runtime maps them to focus changes and hit tests, and keeps a small keymap so shortcuts stay discoverable.\n\n1. Parse bytes into events.\n2. Route the event to the focused widget.\n3. Redraw only what changed.\n\n---\n\nRead the *architecture overview* next, or jump to the [widget gallery](https://example.com/gallery).\n",
    ),
    (
        "Layout",
        "# Layout\n\nA layout splits an area into rectangles from a list of **constraints**. The solver is exact for the common cases and falls back to a general solver only when it must.\n\n## Constraints\n\n| constraint | meaning |\n|:--|--:|\n| `Length(n)` | exactly n cells |\n| `Min(n)` | at least n cells |\n| `Percentage(p)` | p percent of the axis |\n| `Fill(w)` | a weighted share of the rest |\n\n## Flex\n\n- `Start`, `Center`, and `End` place the group\n- `SpaceBetween` spreads the gaps\n  - a lone segment sits at the start\n- `Legacy` keeps the older rules\n\n```rust\nlet [left, right] = Layout::horizontal([Length(24), Fill(1)])\n    .split(area)\n    .into_array();\n```\n\n> Floors and caps are opt in: turn on the `size-limits` feature to bound a segment.\n\n## Wrapping\n\nSegments that do not fit move onto the next line, so a row of tags or buttons reflows when the window narrows. Lines break greedily: a segment joins the current line when its natural size plus the spacing fits in what is left.\n\n1. Measure each segment.\n2. Break the list into lines.\n3. Split every line with the same solver.\n\n## Spacing\n\nOne `gap` setting spaces segments and lines alike, which keeps grids and toolbars consistent. Negative spacing overlaps neighbours by a cell, which is how bordered panels share an edge.\n\n- [x] horizontal and vertical splits\n- [x] wrapped lines\n- [ ] nested constraints in one pass\n\n---\n\nThe solver never allocates on the fast path.\n",
    ),
    (
        "Widgets",
        "# Widgets\n\nThe widget set covers text, lists, tables, gauges, charts, and now **Markdown**.\n\n## Markdown\n\nThe view lays a parsed document out at the width it is given: headings get rules, lists keep a hanging indent, quotes keep their bar, and tables are boxed. ~~Raw source~~ is never shown.\n\n1. Parse once with `markdown::parse`.\n2. Render with `MarkdownView::new(&document)`.\n3. Scroll with `.scroll(rows)` and size a scrollbar from `content_height`.\n\n### Inline styles\n\nText can be *emphasised*, **strong**, ***both***, ~~struck~~, or `code`, and a [link](https://example.com) is underlined.\n\n| widget | feature |\n|:--|:-:|\n| `Sparkline` | sparkline |\n| `BarChart` | barchart |\n| `Chart` | chart |\n| `MarkdownView` | markdown |\n\n## Lists and tasks\n\n- [x] headings, lists, quotes\n- [x] code blocks and tables\n- [x] task items\n  - nested items indent\n    - and nest again\n\n## Quotes\n\n> A quote keeps its bar on every row, even when the text wraps to a second or a third line.\n>\n> > Quotes nest as deep as the text needs.\n\n## Code\n\n```rust\nlet document = markdown::parse(source, Options::gfm());\nMarkdownView::new(&document)\n    .scroll(offset)\n    .render(area, &mut cx);\n```\n\n---\n\nThat is the whole tour.\n",
    ),
];

fn md_documents() -> &'static [lunatui_text::markdown::Document] {
    static DOCUMENTS: std::sync::OnceLock<Vec<lunatui_text::markdown::Document>> =
        std::sync::OnceLock::new();
    DOCUMENTS.get_or_init(|| {
        MD_PAGES
            .iter()
            .map(|(_, source)| {
                lunatui_text::markdown::parse(source, lunatui_text::markdown::Options::gfm())
            })
            .collect()
    })
}

fn markdown_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let text = theme.resolve(Token::TextPrimary);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let raised = cx.token(Token::Surface1);
    let accent = cx.token(Token::Accent);
    let fill = Style::new().fg(text).bg(surface);
    let page = usize::try_from(tick / 80).unwrap_or(0) % MD_PAGES.len();
    let (title, _) = MD_PAGES[page];
    let document = &md_documents()[page];
    StatusBar::new(format!(" Docs  ›  {title}"))
        .style(
            Style::new()
                .fg(accent)
                .bg(raised)
                .with_attrs(Attrs::new().set_bold(true)),
        )
        .render(Rect::new(area.x, area.y, area.width, 1), &mut cx);
    let body = Rect::new(
        area.x,
        area.y + 1,
        area.width,
        area.height.saturating_sub(2),
    );
    let side_w = 26.min(body.width / 3);
    let side = Rect::new(body.x, body.y, side_w, body.height);
    let panel = |title: &str| {
        Block::bordered()
            .title(format!(" {title} "))
            .style(fill)
            .border_style(Style::new().fg(raised).bg(surface))
            .title_style(fill)
    };
    let side_block = panel("Pages");
    side_block.render(side, &mut cx);
    let names: Vec<Vec<String>> = MD_PAGES
        .iter()
        .map(|(name, _)| vec![(*name).to_owned()])
        .collect();
    let source = lunatui_widgets::VecSource::from_rows(names);
    List::new(source)
        .style(fill)
        .highlight_style(
            Style::new()
                .fg(accent)
                .bg(raised)
                .with_attrs(Attrs::new().set_bold(true)),
        )
        .selected(Some(page))
        .render(side_block.inner(side), &mut cx);
    let main = Rect::new(body.x + side_w, body.y, body.width - side_w, body.height);
    let main_block = panel("Reader");
    main_block.render(main, &mut cx);
    let inner = main_block.inner(main);
    let view = Rect::new(
        inner.x + 1,
        inner.y,
        inner.width.saturating_sub(3),
        inner.height,
    );
    let content = MarkdownView::new(document).content_height(view.width);
    let travel = content.saturating_sub(usize::from(view.height));
    let phase = usize::try_from(tick % 80)
        .unwrap_or(0)
        .saturating_sub(6)
        .min(68);
    let scroll = travel * phase / 68;
    MarkdownView::new(document)
        .background(fill)
        .scroll(scroll)
        .render(view, &mut cx);
    Scrollbar::vertical(content, usize::from(view.height), scroll)
        .track_style(Style::new().fg(raised).bg(surface))
        .thumb_style(Style::new().fg(accent).bg(surface))
        .render(
            Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height),
            &mut cx,
        );
    let percent = (scroll * 100).checked_div(travel).unwrap_or(100);
    StatusBar::new(format!(" tab pages   ↑↓ scroll   q quit   {percent}%"))
        .style(
            Style::new()
                .fg(text)
                .bg(raised)
                .with_attrs(Attrs::new().set_dim(true)),
        )
        .render(
            Rect::new(area.x, area.y + area.height - 1, area.width, 1),
            &mut cx,
        );
}

fn markdown_baseline(buffer: &mut Buffer, area: RatRect, _theme: &Theme, tick: u32) {
    if let Some(case) = CASES.iter().find(|case| case.name == "markdown") {
        (case.ratatui)(buffer, area, tick);
    }
}

const TOAST_TICK: Duration = Duration::from_millis(50);
const TOAST_SCRIPT: [(u32, u32, ToastScope, ToastKind, &str); 7] = [
    (
        20,
        70,
        ToastScope::Pane(1),
        ToastKind::Success,
        "api 4.12 deployed",
    ),
    (
        45,
        80,
        ToastScope::Pane(1),
        ToastKind::Info,
        "web: 3 checks queued",
    ),
    (
        70,
        90,
        ToastScope::Pane(2),
        ToastKind::Warning,
        "Log stream is lagging",
    ),
    (
        95,
        60,
        ToastScope::Status,
        ToastKind::Info,
        "Syncing with origin",
    ),
    (
        110,
        100,
        ToastScope::Pane(1),
        ToastKind::Error,
        "jobs: migration failed",
    ),
    (
        130,
        50,
        ToastScope::Status,
        ToastKind::Success,
        "Synced 14 files",
    ),
    (
        150,
        70,
        ToastScope::Pane(2),
        ToastKind::Info,
        "Tailing build 4821",
    ),
];

fn toast_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let text = theme.resolve(Token::TextPrimary);
    let accent = theme.resolve(Token::Accent);
    let raised = theme.resolve(Token::Surface1);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let fill = Style::new().fg(text).bg(surface);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let body = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(1));
    let left = Rect::new(body.x, body.y, body.width / 2, body.height);
    let right = Rect::new(
        left.x + left.width,
        body.y,
        body.width - left.width,
        body.height,
    );
    for (title, pane) in [(" Pipelines ", left), (" Activity ", right)] {
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(title)
            .style(fill)
            .border_style(dim)
            .title_style(Style::new().fg(accent).bg(surface))
            .render(pane, &mut cx);
    }
    let pipelines = [
        "api", "web", "jobs", "edge", "docs", "search", "billing", "auth",
    ];
    for (row, name) in (0_u16..).zip(pipelines) {
        let y = left.y + 1 + row;
        if y + 1 >= left.y + left.height {
            break;
        }
        let state = if name == "jobs" && tick >= 110 {
            "failed "
        } else {
            "passing"
        };
        let line = format!(
            " {name:<10}{state}  #{}",
            4800 + u32::from(row) * 3 + tick / 40
        );
        cx.put_str(
            left.x + 1,
            y,
            clip_display_prefix(&line, left.width.saturating_sub(2)),
            fill,
        );
    }
    for row in 0..right.height.saturating_sub(2) {
        let second = tick / 4 + u32::from(row);
        let line = format!(
            " 12:{:02}:{:02}  worker-{}  ok",
            (second / 60) % 60,
            second % 60,
            row % 5 + 1
        );
        cx.put_str(
            right.x + 1,
            right.y + 1 + row,
            clip_display_prefix(&line, right.width.saturating_sub(2)),
            dim,
        );
    }
    StatusBar::new(" main  ●  3 workers  ●  q quit")
        .style(Style::new().fg(text).bg(raised))
        .render(
            Rect::new(area.x, area.y + body.height, area.width, 1),
            &mut cx,
        );
    let now = TOAST_TICK * tick;
    let mut stack = ToastStack::new();
    for (start, ttl, scope, kind, message) in TOAST_SCRIPT {
        if tick >= start {
            stack.push(
                TOAST_TICK * start,
                Some(TOAST_TICK * ttl),
                scope,
                kind,
                message,
            );
        }
    }
    let panes = |id: u32| match id {
        1 => Some(left),
        2 => Some(right),
        _ => None,
    };
    stack.render(now, area, panes, &mut cx);
}

fn toast_baseline(buffer: &mut Buffer, area: RatRect, _theme: &Theme, tick: u32) {
    if let Some(case) = CASES.iter().find(|case| case.name == "toast") {
        (case.ratatui)(buffer, area, tick);
    }
}

struct SplitMove {
    from: u32,
    to: u32,
    divider: usize,
    target: (i32, i32),
}

const SPLIT_MOVES: [SplitMove; 4] = [
    SplitMove {
        from: 10,
        to: 50,
        divider: 0,
        target: (42, 0),
    },
    SplitMove {
        from: 60,
        to: 100,
        divider: 1,
        target: (0, 19),
    },
    SplitMove {
        from: 110,
        to: 145,
        divider: 2,
        target: (84, 0),
    },
    SplitMove {
        from: 205,
        to: 235,
        divider: 0,
        target: (28, 0),
    },
];

fn lerp(start: i32, end: i32, spent: u32, span: u32) -> i32 {
    let delta = i64::from(end) - i64::from(start);
    let step = delta * i64::from(spent) / i64::from(span.max(1));
    i32::try_from(i64::from(start) + step).unwrap_or(start)
}

fn split_scene(area: Rect, tick: u32) -> (SplitTree, SplitDrag) {
    let (mut tree, files) = SplitTree::new();
    let editor = tree
        .split(files, SplitAxis::Horizontal, 250)
        .unwrap_or(files);
    tree.split(editor, SplitAxis::Vertical, 700);
    tree.split(editor, SplitAxis::Horizontal, 720);
    for divider in 0..3 {
        tree.set_min(DividerId(divider), 12, 12);
    }
    let mut drag = SplitDrag::new();
    let mut start = (0_i32, 0_i32);
    for now in 0..=tick {
        for step in &SPLIT_MOVES {
            if now == step.from {
                let layout = tree.layout(area);
                if let Some(line) = layout.dividers.get(step.divider) {
                    let (x, y) = (line.rect.x, line.rect.y + line.rect.height / 2);
                    start = (i32::from(x), i32::from(y));
                    drag.press(&tree, &layout, x, y);
                }
            } else if now > step.from && now <= step.to {
                let span = step.to - step.from;
                let spent = now - step.from;
                let column = u16::try_from(lerp(start.0, step.target.0, spent, span)).unwrap_or(0);
                let row = u16::try_from(lerp(start.1, step.target.1, spent, span)).unwrap_or(0);
                drag.drag(&mut tree, column, row);
                if now == step.to {
                    drag.release();
                }
            }
        }
    }
    (tree, drag)
}

fn split_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let raised = theme.resolve(Token::Surface1);
    let text = theme.resolve(Token::TextPrimary);
    let accent = theme.resolve(Token::Accent);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let fill = Style::new().fg(text).bg(surface);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let body = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(1));
    let (mut tree, drag) = split_scene(body, tick);
    if (150..195).contains(&tick) {
        tree.zoom(Some(PaneId(1)));
    }
    let layout = tree.layout(body);
    let titles = ["Files", "Editor", "Terminal", "Inspector"];
    let content: [&[&str]; 4] = [
        &[
            "src/",
            "  block.rs",
            "  split.rs",
            "  tabs.rs",
            "tests/",
            "Cargo.toml",
            "README.md",
        ],
        &[
            "fn layout(&self, area: Rect) -> SplitLayout {",
            "    let mut layout = SplitLayout::default();",
            "    place(&self.root, area, &mut 0, &mut layout);",
            "    layout",
            "}",
        ],
        &[
            "$ cargo test -p lunatui-widgets",
            "   Compiling lunatui-widgets",
            "test result: ok. 29 passed",
        ],
        &["panes     4", "dividers  3", "zoomed    none"],
    ];
    for area in &layout.panes {
        let index = usize::try_from(area.pane.0).unwrap_or(0).min(3);
        let title = if tree.zoomed().is_some() {
            format!(" {} (zoomed) ", titles[index])
        } else {
            format!(" {} ", titles[index])
        };
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(title)
            .style(fill)
            .border_style(dim)
            .title_style(Style::new().fg(accent).bg(surface))
            .render(area.rect, &mut cx);
        let room = area.rect.width.saturating_sub(4);
        for (row, line) in (0_u16..).zip(content[index]) {
            if row + 2 >= area.rect.height {
                break;
            }
            cx.put_str(
                area.rect.x + 2,
                area.rect.y + 1 + row,
                clip_display_prefix(line, room),
                fill,
            );
        }
    }
    layout.render_dividers(
        dim,
        drag.active(),
        Style::new().fg(accent).bg(surface),
        &mut cx,
    );
    StatusBar::new(" drag a divider to resize  ●  zoom with z")
        .style(Style::new().fg(text).bg(raised))
        .render(
            Rect::new(area.x, area.y + body.height, area.width, 1),
            &mut cx,
        );
}

fn split_baseline(buffer: &mut Buffer, area: RatRect, _theme: &Theme, tick: u32) {
    if let Some(case) = CASES.iter().find(|case| case.name == "split") {
        (case.ratatui)(buffer, area, tick);
    }
}

const TAB_TITLES: [&str; 5] = ["Overview", "Builds", "Deploys", "Logs", "Settings"];
const TAB_HOVER: u32 = 10;
const TAB_LIFT: u32 = 6;
const TAB_HOLD: u32 = 8;
const TAB_SETTLE: u32 = 9;

struct TabDragStep {
    from: u32,
    to: u32,
    grab: usize,
    drop: usize,
}

const TAB_STEPS: [TabDragStep; 3] = [
    TabDragStep {
        from: 24,
        to: 80,
        grab: 1,
        drop: 3,
    },
    TabDragStep {
        from: 105,
        to: 155,
        grab: 1,
        drop: 0,
    },
    TabDragStep {
        from: 180,
        to: 228,
        grab: 2,
        drop: 4,
    },
];

const BUILDS: [(&str, &str, &str, &str, &str, u32); 10] = [
    (
        "4820",
        "feature/split-tree",
        "9be2d71",
        "Resize panes with the pointer",
        "passed",
        252,
    ),
    (
        "4819",
        "main",
        "5f03a8c",
        "Clip list text before its mark",
        "passed",
        198,
    ),
    (
        "4818",
        "fix/toast-expiry",
        "c47e10b",
        "Expire toasts on the virtual clock",
        "failed",
        87,
    ),
    (
        "4817",
        "main",
        "e81a4f2",
        "Add Canvas quadrant markers",
        "passed",
        231,
    ),
    (
        "4816",
        "feature/canvas",
        "02d6b9e",
        "Draw circles with sextants",
        "passed",
        244,
    ),
    (
        "4815",
        "docs/porting",
        "7ac3e51",
        "Explain the theme tokens",
        "passed",
        64,
    ),
    (
        "4814",
        "main",
        "b3f90d7",
        "Keep the image ring background",
        "passed",
        205,
    ),
    (
        "4813",
        "fix/scroll-edge",
        "4d2c8a6",
        "Clip popups at the grid edge",
        "canceled",
        31,
    ),
    (
        "4812",
        "main",
        "f17e0c3",
        "Add the BarChart widget",
        "passed",
        219,
    ),
    (
        "4811",
        "feature/barchart",
        "6e5b2d4",
        "Label grouped bars",
        "passed",
        226,
    ),
];

const BUILD_STEPS: [&str; 6] = ["checkout", "build", "clippy", "test", "bench", "package"];

const DEPLOYS: [(&str, &str, &str, u32, &str); 9] = [
    ("web", "prod", "v2.14.0", 4, "14m"),
    ("worker", "prod", "v2.13.2", 8, "3h"),
    ("edge", "prod", "v2.14.0", 12, "16m"),
    ("scheduler", "prod", "v1.9.4", 2, "2d"),
    ("billing", "prod", "v3.2.1", 3, "5d"),
    ("api", "staging", "v2.15.0-rc.1", 2, "41m"),
    ("web", "staging", "v2.15.0-rc.1", 2, "41m"),
    ("worker", "staging", "v2.15.0-rc.1", 4, "40m"),
    ("edge", "staging", "v2.15.0-rc.1", 3, "39m"),
];

const LOG_SOURCES: [&str; 5] = ["api", "worker-2", "edge", "scheduler", "billing"];

struct TabInk {
    fill: Style,
    dim: Style,
    muted: Style,
    header: Style,
    active: Style,
    frame: Style,
    marker: Style,
    lifted: Style,
    lifted_title: Style,
    pointer: Style,
    status: Style,
    status_key: Style,
    ok: Style,
    warn: Style,
    fail: Style,
    gauge: Style,
    track: Style,
    soft: Style,
    lead: Style,
}

impl TabInk {
    fn new(theme: &Theme) -> Self {
        let surface = theme.resolve(Token::Surface0);
        let raised = theme.resolve(Token::Surface1);
        let text = theme.resolve(Token::TextPrimary);
        let accent = theme.resolve(Token::Accent);
        let bold = Attrs::new().set_bold(true);
        let fill = Style::new().fg(text).bg(surface);
        Self {
            fill,
            dim: fill.with_attrs(Attrs::new().set_dim(true)),
            muted: fill.with_attrs(Attrs::new().set_dim(true)),
            header: fill.with_attrs(Attrs::new().set_dim(true)),
            active: Style::new().fg(accent).bg(surface).with_attrs(bold),
            frame: Style::new().fg(accent).bg(surface),
            marker: Style::new()
                .fg(accent)
                .bg(surface)
                .with_attrs(Attrs::new().set_dim(true)),
            lifted: Style::new().fg(accent).bg(raised).with_attrs(bold),
            lifted_title: Style::new().fg(text).bg(raised).with_attrs(bold),
            pointer: Style::new().fg(text).bg(surface).with_attrs(bold),
            status: Style::new().fg(text).bg(raised),
            status_key: Style::new().fg(accent).bg(raised).with_attrs(bold),
            ok: Style::new().fg(theme.resolve(Token::Ok)),
            warn: Style::new().fg(theme.resolve(Token::Warn)),
            fail: Style::new().fg(theme.resolve(Token::Fail)),
            gauge: Style::new().fg(accent).bg(raised),
            track: Style::new().bg(raised),
            soft: Style::new().with_attrs(Attrs::new().set_dim(true)),
            lead: Style::new().fg(accent).with_attrs(bold),
        }
    }
}

struct TabGhost {
    title: &'static str,
    x: u16,
    lift: u16,
}

struct TabDragFrame {
    titles: Vec<&'static str>,
    shown: Vec<&'static str>,
    selected: usize,
    hole: Option<usize>,
    drag: TabsDrag,
    ghost: Option<TabGhost>,
    pointer: Option<u16>,
    moved: Option<(&'static str, usize)>,
}

fn tab_slot_width(title: &str) -> u16 {
    u16::try_from(cols(title)).unwrap_or(0).saturating_add(4)
}

fn tab_strip_areas(titles: &[&'static str], strip: Rect) -> Vec<(usize, Rect)> {
    Tabs::new(titles.iter().copied())
        .boxed(Style::new(), Style::new())
        .tab_areas(strip)
}

fn tab_slot(areas: &[(usize, Rect)], index: usize) -> Rect {
    areas
        .iter()
        .find(|(i, _)| *i == index)
        .map_or(Rect::new(0, 0, 0, 0), |(_, rect)| *rect)
}

fn tab_center(areas: &[(usize, Rect)], index: usize) -> u16 {
    let slot = tab_slot(areas, index);
    slot.x + slot.width / 2
}

fn eased(start: u16, end: u16, spent: u32, span: u32) -> u16 {
    let span = i64::from(span.max(1));
    let spent = i64::from(spent).min(span);
    let curve = spent * spent * (3 * span - 2 * spent);
    let (start, end) = (i64::from(start), i64::from(end));
    u16::try_from(start + (end - start) * curve / (span * span * span)).unwrap_or(0)
}

fn tab_drag_frame(strip: Rect, tick: u32) -> TabDragFrame {
    let row = strip.y + 1;
    let mut titles = TAB_TITLES.to_vec();
    let mut selected = 3_usize;
    let mut drag = TabsDrag::new();
    let mut column = 0_u16;
    let mut pressed = 0_u32;
    let mut released: Option<(u32, usize, u16)> = None;
    for now in 0..=tick {
        for step in &TAB_STEPS {
            let areas = tab_strip_areas(&titles, strip);
            if now == step.from {
                column = tab_center(&areas, step.grab);
                drag.press(&areas, column, row);
                selected = step.grab;
                pressed = now;
                released = None;
            } else if now > step.from && now <= step.to {
                let travel = step.from + TAB_LIFT;
                let span = (step.to - TAB_HOLD).saturating_sub(travel);
                column = eased(
                    tab_center(&areas, step.grab),
                    tab_center(&areas, step.drop),
                    now.saturating_sub(travel),
                    span,
                );
                drag.drag(&areas, column, row);
                if now == step.to
                    && let Some(reorder) = drag.release()
                {
                    reorder.apply(&mut titles);
                    selected = reorder.selection(selected);
                    released = Some((now, reorder.to, column));
                }
            }
        }
    }
    let mut frame = TabDragFrame {
        shown: titles.clone(),
        titles,
        selected,
        hole: None,
        drag,
        ghost: None,
        pointer: None,
        moved: None,
    };
    if let Some(from) = drag.from() {
        let to = drag.target().unwrap_or(from);
        TabMove { from, to }.apply(&mut frame.shown);
        let title = frame.titles.get(from).copied().unwrap_or("");
        frame.hole = Some(to);
        frame.ghost = Some(TabGhost {
            title,
            x: column.saturating_sub(tab_slot_width(title) / 2),
            lift: u16::from(tick >= pressed + 2),
        });
        frame.pointer = Some(column);
        return frame;
    }
    if let Some((at, to, from_column)) = released {
        let title = frame.titles.get(to).copied().unwrap_or("");
        frame.moved = Some((title, to));
        let spent = tick - at;
        if spent < TAB_SETTLE {
            let rest = tab_slot(&tab_strip_areas(&frame.titles, strip), to).x;
            let start = from_column.saturating_sub(tab_slot_width(title) / 2);
            frame.hole = Some(to);
            frame.ghost = Some(TabGhost {
                title,
                x: eased(start, rest, spent, TAB_SETTLE - 3),
                lift: u16::from(spent < TAB_SETTLE - 3),
            });
        }
    }
    let next = TAB_STEPS
        .iter()
        .find(|step| step.from > tick && step.from - tick <= TAB_HOVER);
    if let Some(step) = next {
        frame.pointer = Some(tab_center(
            &tab_strip_areas(&frame.titles, strip),
            step.grab,
        ));
    }
    frame
}

fn draw_tab_strip(frame: &TabDragFrame, strip: Rect, ink: &TabInk, cx: &mut RenderCx<'_>) {
    let labels = frame.shown.iter().enumerate().map(|(index, title)| {
        if frame.hole == Some(index) {
            " ".repeat(cols(title))
        } else {
            (*title).to_string()
        }
    });
    let (select, box_style) = match frame.hole {
        Some(hole) => (hole, ink.marker),
        None => (frame.selected, ink.frame),
    };
    Tabs::new(labels)
        .select(select)
        .style(ink.dim)
        .highlight_style(ink.active)
        .boxed(box_style, ink.dim)
        .render(strip, cx);
    if let Some(ghost) = &frame.ghost {
        let width = tab_slot_width(ghost.title);
        let y = strip.y.saturating_sub(ghost.lift);
        Block::bordered()
            .border_type(BorderType::Rounded)
            .style(ink.lifted_title)
            .border_style(ink.lifted)
            .render(Rect::new(ghost.x, y, width, 3), cx);
        Paragraph::new(format!(" {} ", ghost.title))
            .style(ink.lifted_title)
            .render(Rect::new(ghost.x + 1, y + 1, width - 2, 1), cx);
    }
    if let Some(column) = frame.pointer {
        Paragraph::new("↖")
            .style(ink.pointer)
            .render(Rect::new(column, strip.y + 2, 1, 1), cx);
    }
}

fn duration_label(seconds: u32) -> String {
    format!("{}m{:02}s", seconds / 60, seconds % 60)
}

fn status_style(status: &str, ink: &TabInk) -> Style {
    match status {
        "passed" | "healthy" => ink.ok,
        "failed" | "degraded" => ink.fail,
        "running" | "rolling" => ink.warn,
        _ => ink.soft,
    }
}

fn tick_ratio(tick: u32, start: u32, span: u32) -> f32 {
    let spent = u16::try_from(tick.saturating_sub(start).min(span)).unwrap_or(0);
    f32::from(spent) / f32::from(u16::try_from(span.max(1)).unwrap_or(u16::MAX))
}

fn progress_gauge(title: String, ratio: f32, area: Rect, ink: &TabInk, cx: &mut RenderCx<'_>) {
    Gauge::new(ratio)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(title)
                .style(ink.fill)
                .border_style(ink.dim)
                .title_style(ink.header),
        )
        .gauge_style(ink.gauge)
        .track_style(ink.track)
        .sub_cell(true)
        .render(area, cx);
}

fn builds_body(area: Rect, tick: u32, ink: &TabInk, cx: &mut RenderCx<'_>) {
    let ratio = 0.3 + 0.7 * tick_ratio(tick, 0, 240);
    let done = ratio >= 1.0;
    let step = BUILD_STEPS
        .get(usize::try_from(tick * 6 / 241).unwrap_or(0))
        .copied()
        .unwrap_or("package");
    let running_status = if done { "passed" } else { "running" };
    let mut rows = vec![vec![
        "#4821".to_string(),
        "feature/tabs-drag".to_string(),
        "882be46".to_string(),
        "Reorder tabs with the pointer".to_string(),
        running_status.to_string(),
        duration_label(151 + tick.min(240) / 2),
    ]];
    let mut styles = vec![vec![
        ink.lead,
        Style::new(),
        ink.soft,
        Style::new(),
        status_style(running_status, ink),
        Style::new(),
    ]];
    for (build, branch, commit, message, status, seconds) in BUILDS {
        rows.push(vec![
            format!("#{build}"),
            branch.to_string(),
            commit.to_string(),
            message.to_string(),
            status.to_string(),
            duration_label(seconds),
        ]);
        styles.push(vec![
            Style::new(),
            Style::new(),
            ink.soft,
            Style::new(),
            status_style(status, ink),
            Style::new(),
        ]);
    }
    let table_area = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(4));
    Table::new(
        lunatui_widgets::VecSource::from_rows(rows),
        vec![
            Constraint::Length(7),
            Constraint::Length(20),
            Constraint::Length(9),
            Constraint::Fill(1),
            Constraint::Length(9),
            Constraint::Length(7),
        ],
    )
    .header(
        ["Build", "Branch", "Commit", "Message", "Status", "Time"].map(|title| (title, ink.header)),
    )
    .style(ink.fill)
    .alignments(vec![
        Alignment::Left,
        Alignment::Left,
        Alignment::Left,
        Alignment::Left,
        Alignment::Left,
        Alignment::Right,
    ])
    .odd_row_style(ink.track)
    .cell_styles(styles)
    .render(table_area, cx);
    let percent = u32::from(u16::try_from(tick.min(240) * 70 / 240).unwrap_or(0)) + 30;
    let label = if done {
        " #4821 feature/tabs-drag  passed ".to_string()
    } else {
        format!(" #4821 feature/tabs-drag  {step}  {percent}% ")
    };
    let gauge_area = Rect::new(
        area.x,
        area.y + area.height.saturating_sub(3),
        area.width,
        3,
    );
    progress_gauge(label, ratio, gauge_area, ink, cx);
}

fn deploys_body(area: Rect, tick: u32, ink: &TabInk, cx: &mut RenderCx<'_>) {
    let ready = (1 + tick.saturating_sub(80) / 14).min(6);
    let rolling = if ready < 6 { "rolling" } else { "healthy" };
    let mut rows = vec![vec![
        "api".to_string(),
        "prod".to_string(),
        "v2.14.0".to_string(),
        format!("{ready}/6"),
        rolling.to_string(),
        format!("{}s", 20 + tick / 3),
    ]];
    let mut styles = vec![vec![
        ink.lead,
        Style::new(),
        Style::new(),
        Style::new(),
        status_style(rolling, ink),
        Style::new(),
    ]];
    for (index, (service, env, version, replicas, age)) in (0_u32..).zip(DEPLOYS) {
        let up = if index == 1 { replicas - 1 } else { replicas };
        let status = if up < replicas { "degraded" } else { "healthy" };
        rows.push(vec![
            service.to_string(),
            env.to_string(),
            version.to_string(),
            format!("{up}/{replicas}"),
            status.to_string(),
            age.to_string(),
        ]);
        styles.push(vec![
            Style::new(),
            if env == "prod" {
                Style::new()
            } else {
                ink.soft
            },
            Style::new(),
            Style::new(),
            status_style(status, ink),
            Style::new(),
        ]);
    }
    let table_area = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(4));
    Table::new(
        lunatui_widgets::VecSource::from_rows(rows),
        vec![
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(15),
            Constraint::Length(9),
            Constraint::Length(10),
            Constraint::Fill(1),
        ],
    )
    .header(
        ["Service", "Env", "Version", "Ready", "Status", "Age"].map(|title| (title, ink.header)),
    )
    .style(ink.fill)
    .alignments(vec![
        Alignment::Left,
        Alignment::Left,
        Alignment::Left,
        Alignment::Left,
        Alignment::Left,
        Alignment::Right,
    ])
    .odd_row_style(ink.track)
    .cell_styles(styles)
    .render(table_area, cx);
    let label = format!(" api prod  v2.13.2 → v2.14.0  {ready}/6 ready ");
    let ratio = f32::from(u16::try_from(ready).unwrap_or(0)) / 6.0;
    let gauge_area = Rect::new(
        area.x,
        area.y + area.height.saturating_sub(3),
        area.width,
        3,
    );
    progress_gauge(label, ratio, gauge_area, ink, cx);
}

fn log_line(index: u32) -> (String, &'static str, &'static str, String) {
    let tenths = 24_000 + index * 37 / 10;
    let time = format!(
        "12:{:02}:{:02}.{}",
        tenths / 600,
        tenths / 10 % 60,
        tenths % 10
    );
    let source = LOG_SOURCES[usize::try_from(index % 5).unwrap_or(0)];
    let (level, message) = if index % 19 == 11 {
        (
            "ERROR",
            format!(
                "deploy hook timed out after 30s (attempt {})",
                index % 3 + 1
            ),
        )
    } else if index % 7 == 3 {
        (
            "WARN",
            format!("slow query on builds: {}ms", 240 + index % 9 * 31),
        )
    } else {
        let message = match index % 6 {
            0 => format!("GET /v1/builds 200 {}ms", 12 + index % 23),
            1 => format!(
                "job {} finished in {}.{}s",
                8800 + index,
                1 + index % 4,
                index % 10
            ),
            2 => format!("cache hit ratio 0.9{}", index % 10),
            3 => format!("POST /v1/deploys 201 {}ms", 40 + index % 37),
            4 => format!("pool connections {}/32", 10 + index % 17),
            _ => format!("health check ok in {}ms", 2 + index % 7),
        };
        ("INFO", message)
    };
    (time, level, source, message)
}

fn logs_body(area: Rect, tick: u32, ink: &TabInk, cx: &mut RenderCx<'_>) {
    let count = 60 + tick / 5;
    let visible = u32::from(area.height.saturating_sub(1));
    let mut rows = Vec::new();
    let mut styles = Vec::new();
    for index in count.saturating_sub(visible)..count {
        let (time, level, source, message) = log_line(index);
        let level_style = match level {
            "ERROR" => ink.fail,
            "WARN" => ink.warn,
            _ => ink.ok,
        };
        rows.push(vec![time, level.to_string(), source.to_string(), message]);
        styles.push(vec![ink.soft, level_style, ink.soft, Style::new()]);
    }
    Table::new(
        lunatui_widgets::VecSource::from_rows(rows),
        vec![
            Constraint::Length(12),
            Constraint::Length(7),
            Constraint::Length(11),
            Constraint::Fill(1),
        ],
    )
    .header(["Time", "Level", "Source", "Message"].map(|title| (title, ink.header)))
    .style(ink.fill)
    .cell_styles(styles)
    .render(area, cx);
}

fn tabs_drag_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let ink = TabInk::new(theme);
    grid.fill(area, ink.fill, ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    Paragraph::new(" Release pipelines")
        .style(ink.header)
        .render(Rect::new(area.x, area.y, area.width, 1), &mut cx);
    let strip = Rect::new(area.x + 2, area.y + 2, area.width.saturating_sub(4), 3);
    let frame = tab_drag_frame(strip, tick);
    let body = Rect::new(
        area.x + 2,
        strip.y + 3,
        area.width.saturating_sub(4),
        area.height.saturating_sub(strip.y + 3 - area.y + 1),
    );
    let name = frame.titles.get(frame.selected).copied().unwrap_or("");
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(format!(" {name} "))
        .style(ink.fill)
        .border_style(ink.dim)
        .title_style(ink.active);
    let inner = block.inner(body);
    block.render(body, &mut cx);
    let content = Rect::new(
        inner.x + 1,
        inner.y,
        inner.width.saturating_sub(2),
        inner.height,
    );
    match name {
        "Builds" => builds_body(content, tick, &ink, &mut cx),
        "Deploys" => deploys_body(content, tick, &ink, &mut cx),
        "Logs" => logs_body(content, tick, &ink, &mut cx),
        _ => Paragraph::new("5 services healthy, 1 rolling out")
            .style(ink.muted)
            .render(content, &mut cx),
    }
    draw_tab_strip(&frame, strip, &ink, &mut cx);
    let hint = match (frame.drag.from(), frame.drag.target(), frame.moved) {
        (Some(from), Some(target), _) => format!(
            " moving {} to position {} of {}   release to drop",
            frame.titles.get(from).copied().unwrap_or(""),
            target + 1,
            frame.titles.len()
        ),
        (Some(_), None, _) => " release to cancel".to_string(),
        (None, _, Some((title, to))) => format!(" moved {title} to position {}", to + 1),
        (None, _, None) => " drag a tab to reorder".to_string(),
    };
    StatusBar::new(hint)
        .style(ink.status)
        .left([(" main ", ink.status_key)])
        .right([(" 6 services  3 workers ", ink.status)])
        .render(
            Rect::new(
                area.x,
                area.y + area.height.saturating_sub(1),
                area.width,
                1,
            ),
            &mut cx,
        );
}

fn tabs_drag_baseline(buffer: &mut Buffer, area: RatRect, _theme: &Theme, tick: u32) {
    if let Some(case) = CASES.iter().find(|case| case.name == "tabs") {
        (case.ratatui)(buffer, area, tick);
    }
}

const CRUMB_PATH: [&str; 6] = [
    "Home",
    "Projects",
    "Moonbase2090",
    "lunatui",
    "lunatui-widgets",
    "src",
];

type CrumbEntry = (&'static str, u32, &'static str);

const CRUMB_DIRS: [&[CrumbEntry]; 6] = [
    &[
        ("Documents/", 0, "Oct 12"),
        ("Downloads/", 0, "Oct 27"),
        ("Pictures/", 0, "Sep 30"),
        ("Projects/", 0, "Oct 28"),
        ("notes.txt", 2_400, "Oct 21"),
        ("todo.md", 1_100, "Oct 26"),
    ],
    &[
        ("Moonbase2090/", 0, "Oct 28"),
        ("Scratch/", 0, "Oct 03"),
        ("archive/", 0, "Aug 19"),
        ("README.md", 3_900, "Oct 14"),
    ],
    &[
        ("lunatui/", 0, "Oct 28"),
        ("prismattyc/", 0, "Oct 27"),
        ("rookrunner/", 0, "Oct 28"),
        ("scorecard/", 0, "Oct 25"),
        ("waypoint/", 0, "Oct 26"),
        ("NOTES.md", 8_200, "Oct 24"),
    ],
    &[
        ("benches/", 0, "Oct 27"),
        ("examples/", 0, "Oct 28"),
        ("lunatui-core/", 0, "Oct 26"),
        ("lunatui-text/", 0, "Oct 26"),
        ("lunatui-theme/", 0, "Oct 20"),
        ("lunatui-widgets/", 0, "Oct 28"),
        ("CHANGELOG.md", 21_700, "Oct 28"),
        ("Cargo.toml", 1_900, "Oct 27"),
        ("README.md", 15_300, "Oct 28"),
    ],
    &[
        ("src/", 0, "Oct 28"),
        ("tests/", 0, "Oct 28"),
        ("Cargo.toml", 4_100, "Oct 28"),
        ("README.md", 6_800, "Oct 28"),
    ],
    &[
        ("block.rs", 18_400, "Oct 24"),
        ("borders.rs", 4_900, "Oct 20"),
        ("breadcrumbs.rs", 8_600, "Oct 28"),
        ("gauge.rs", 12_100, "Oct 26"),
        ("lib.rs", 5_300, "Oct 28"),
        ("list.rs", 3_200, "Oct 26"),
        ("table.rs", 21_900, "Oct 27"),
        ("tabs.rs", 9_700, "Oct 28"),
    ],
];

fn crumb_size(bytes: u32) -> String {
    if bytes == 0 {
        String::from("--")
    } else if bytes < 1_000 {
        format!("{bytes} B")
    } else {
        format!("{}.{} KB", bytes / 1_000, bytes % 1_000 / 100)
    }
}

fn crumb_preview(name: &str, bytes: u32, level: usize) -> Vec<String> {
    let extension = std::path::Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str());
    let kind = match extension {
        _ if name.ends_with('/') => "Folder",
        Some("rs") => "Rust source",
        Some("toml") => "TOML",
        Some("md") => "Markdown",
        _ => "Text",
    };
    let mut lines = vec![
        name.trim_end_matches('/').to_owned(),
        format!("{kind}  ·  {}", crumb_size(bytes)),
        String::new(),
    ];
    if name.ends_with('/') {
        let inside = CRUMB_DIRS.get(level + 1).map_or(0, |dir| dir.len());
        lines.push(format!("{inside} items"));
        lines.push(String::from("Press Enter to open"));
    } else if extension == Some("rs") {
        for line in [
            "// Copyright (c) 2026 Moonbase 2090 LLC",
            "// SPDX-License-Identifier: MIT",
            "",
            "use lunatui_core::{Rect, Size, Style};",
            "",
            "use crate::{RenderCx, Widget};",
            "",
            "#[derive(Clone, Debug, PartialEq, Eq)]",
        ] {
            lines.push(line.to_owned());
        }
    } else if extension == Some("toml") {
        for line in [
            "[package]",
            "name = \"lunatui-widgets\"",
            "version.workspace = true",
            "",
            "[features]",
            "default = []",
            "widget-state = []",
        ] {
            lines.push(line.to_owned());
        }
    } else {
        for line in [
            "# Notes",
            "",
            "- check the release PRs",
            "- review the showcase media",
            "- retarget the stacked branches",
        ] {
            lines.push(line.to_owned());
        }
    }
    lines
}

fn crumb_panes(
    cx: &mut RenderCx<'_>,
    area: Rect,
    level: usize,
    tick: u32,
    styles: [Style; 2],
    colors: [Color; 2],
) {
    let [fill, dim] = styles;
    let [accent, raised] = colors;
    let (text, surface) = (fill.fg, fill.bg);
    let entries = CRUMB_DIRS[level];
    let body_top = area.y;
    let body_height = area.height.min(18);
    let list = Rect::new(area.x + 2, body_top, 72, body_height);
    let side = Rect::new(
        list.x + list.width + 1,
        body_top,
        area.width.saturating_sub(list.width + 5),
        body_height,
    );
    let selected = usize::try_from(tick / 14).unwrap_or(0) % entries.len();
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(format!(" {} ", CRUMB_PATH[level]))
        .style(fill)
        .border_style(dim)
        .title_style(Style::new().fg(accent).bg(surface))
        .render(list, cx);
    let columns = [list.x + 2, list.x + 44, list.x + 58];
    cx.put_str(columns[0], list.y + 1, "Name", dim);
    cx.put_str(columns[1], list.y + 1, "Size", dim);
    cx.put_str(columns[2], list.y + 1, "Modified", dim);
    for (row, (name, bytes, modified)) in (0_u16..).zip(entries.iter()) {
        let y = list.y + 3 + row;
        if y + 1 >= list.y + list.height {
            break;
        }
        let picked = usize::from(row) == selected;
        let style = if picked {
            Style::new().fg(text).bg(raised)
        } else if name.ends_with('/') {
            Style::new().fg(accent).bg(surface)
        } else {
            fill
        };
        cx.put_str(
            list.x + 1,
            y,
            &" ".repeat(usize::from(list.width.saturating_sub(2))),
            style,
        );
        cx.put_str(columns[0], y, name, style);
        cx.put_str(columns[1], y, &crumb_size(*bytes), style);
        cx.put_str(columns[2], y, modified, style);
    }
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Preview ")
        .style(fill)
        .border_style(dim)
        .title_style(Style::new().fg(accent).bg(surface))
        .render(side, cx);
    let (name, bytes, _) = entries[selected];
    for (row, line) in (0_u16..).zip(crumb_preview(name, bytes, level)) {
        let y = side.y + 1 + row;
        if y + 1 >= side.y + side.height {
            break;
        }
        let style = if row == 0 {
            fill.fg(accent)
        } else if row == 1 {
            dim
        } else {
            fill
        };
        cx.put_str(
            side.x + 2,
            y,
            clip_display_prefix(&line, side.width.saturating_sub(4)),
            style,
        );
    }
    let recent = Rect::new(
        area.x + 2,
        body_top + body_height,
        area.width.saturating_sub(4),
        area.height.saturating_sub(body_height),
    );
    crumb_visited(cx, recent, level, styles, colors);
}

fn crumb_visited(
    cx: &mut RenderCx<'_>,
    recent: Rect,
    level: usize,
    styles: [Style; 2],
    colors: [Color; 2],
) {
    let [fill, dim] = styles;
    let [accent, _] = colors;
    let surface = fill.bg;
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Visited ")
        .style(fill)
        .border_style(dim)
        .title_style(Style::new().fg(accent).bg(surface))
        .render(recent, cx);
    for (row, depth) in (0_u16..).zip(1..=level + 1) {
        let y = recent.y + 1 + row;
        if y + 1 >= recent.y + recent.height {
            break;
        }
        let path = CRUMB_PATH[..depth].join("/");
        let count = CRUMB_DIRS[depth - 1].len();
        let here = depth == level + 1;
        let marker = if here { "▶" } else { " " };
        let line = format!(" {marker} {path:<52}{count:>3} items");
        let style = if here { fill.fg(accent) } else { dim };
        cx.put_str(
            recent.x + 1,
            y,
            clip_display_prefix(&line, recent.width.saturating_sub(2)),
            style,
        );
    }
}

fn breadcrumbs_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let raised = theme.resolve(Token::Surface1);
    let text = theme.resolve(Token::TextPrimary);
    let accent = theme.resolve(Token::Accent);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let fill = Style::new().fg(text).bg(surface);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let header = Style::new().fg(text).bg(raised);

    let depth = if tick < 190 {
        (1 + usize::try_from(tick / 28).unwrap_or(0)).min(CRUMB_PATH.len())
    } else {
        2
    };
    let span = area.width.saturating_sub(4);
    let squeeze = if (96..176).contains(&tick) {
        u64::from(span) - tri(tick - 96, 80, 0, u64::from(span.saturating_sub(26)))
    } else {
        u64::from(span)
    };
    let wide = u16::try_from(squeeze).unwrap_or(span).min(span);
    let bar = Rect::new(area.x + 2, area.y + 1, wide, 1);
    let mut crumbs = Breadcrumbs::new(CRUMB_PATH[..depth].iter().copied())
        .style(header)
        .current_style(header.fg(accent).with_attrs(Attrs::new().set_bold(true)))
        .separator_style(header.with_attrs(Attrs::new().set_dim(true)));
    let areas = crumbs.segment_areas(bar);
    let hover = if (150..190).contains(&tick) {
        areas
            .iter()
            .map(|(index, _)| *index)
            .find(|index| *index == 1)
    } else if tick < 150 && tick % 40 >= 24 && !areas.is_empty() {
        let pick = usize::try_from(tick / 40).unwrap_or(0) % areas.len();
        areas.get(pick).map(|(index, _)| *index)
    } else {
        None
    };
    crumbs = crumbs.hover(
        hover,
        Style::new().with_attrs(Attrs::new().set_underline(UnderlineStyle::Single)),
    );
    cx.put_str(
        area.x,
        area.y + 1,
        &" ".repeat(usize::from(area.width)),
        header,
    );
    crumbs.render(bar, &mut cx);
    cx.put_str(area.x + 1, area.y, " Files", fill.fg(accent));

    let level = depth - 1;
    let entries = CRUMB_DIRS[level];
    let panes = Rect::new(
        area.x,
        area.y + 3,
        area.width,
        area.height.saturating_sub(5),
    );
    crumb_panes(&mut cx, panes, level, tick, [fill, dim], [accent, raised]);
    StatusBar::new(format!(
        " {} items  ●  {}  ●  click a path segment to jump back",
        entries.len(),
        CRUMB_PATH[..depth].join("/")
    ))
    .style(Style::new().fg(text).bg(raised))
    .render(
        Rect::new(
            area.x,
            area.y + area.height.saturating_sub(1),
            area.width,
            1,
        ),
        &mut cx,
    );
}

fn breadcrumbs_baseline(buffer: &mut Buffer, area: RatRect, _theme: &Theme, tick: u32) {
    if let Some(case) = CASES.iter().find(|case| case.name == "breadcrumbs") {
        (case.ratatui)(buffer, area, tick);
    }
}

const KH_MOVE: &[KeyChord] = &[KeyChord::new(KeyCode::Down, Modifiers::new())];
const KH_OPEN: &[KeyChord] = &[KeyChord::new(KeyCode::Enter, Modifiers::new())];
const KH_SEARCH: &[KeyChord] = &[KeyChord::new(KeyCode::Char('/'), Modifiers::new())];
const KH_PUBLISH: &[KeyChord] = &[KeyChord::new(KeyCode::Char('p'), Modifiers::new())];
const KH_QUIT: &[KeyChord] = &[KeyChord::new(KeyCode::Char('q'), Modifiers::new())];
const KH_ESC: &[KeyChord] = &[KeyChord::new(KeyCode::Esc, Modifiers::new())];
const KH_CLEAR: &[KeyChord] = &[KeyChord::new(
    KeyCode::Char('u'),
    Modifiers::new().control(),
)];
const KH_YES: &[KeyChord] = &[KeyChord::new(KeyCode::Char('y'), Modifiers::new())];
const KH_NO: &[KeyChord] = &[KeyChord::new(KeyCode::Char('n'), Modifiers::new())];

fn kh_keymap(mode: usize) -> Keymap {
    let mut keymap = Keymap::new();
    let commands: &[Command] = match mode {
        0 => &[
            Command::new("move", "Move", KH_MOVE),
            Command::new("open", "Open log", KH_OPEN),
            Command::new("search", "Search checks", KH_SEARCH),
            Command::new("publish", "Publish", KH_PUBLISH),
            Command::new("quit", "Quit", KH_QUIT),
        ],
        1 => &[
            Command::new("apply", "Apply filter", KH_OPEN),
            Command::new("clear", "Clear", KH_CLEAR),
            Command::new("cancel", "Cancel", KH_ESC),
        ],
        _ => &[
            Command::new("yes", "Publish v0.1.13", KH_YES),
            Command::new("no", "Go back", KH_NO),
            Command::new("cancel", "Cancel", KH_ESC),
        ],
    };
    for command in commands {
        keymap.register(*command);
    }
    keymap
}

const KH_CHECKS: [(&str, &str); 8] = [
    ("fmt", "18 s"),
    ("clippy", "41 s"),
    ("test", "2 m 12 s"),
    ("msrv", "49 s"),
    ("deny", "2 m 22 s"),
    ("docs", "25 s"),
    ("bench", "3 m 32 s"),
    ("scorecard", "1 m 54 s"),
];

fn kh_state(tick: u32) -> (usize, String, usize) {
    let mode = if (150..200).contains(&tick) {
        2
    } else {
        usize::from((60..130).contains(&tick))
    };
    let query = if mode == 1 {
        let typed = usize::try_from((tick - 60) / 14).unwrap_or(0).min(4);
        "test".chars().take(typed.min(4)).collect()
    } else if (130..150).contains(&tick) {
        String::from("test")
    } else {
        String::new()
    };
    (mode, query, usize::try_from(tick / 12).unwrap_or(0))
}

fn key_hints_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let raised = theme.resolve(Token::Surface1);
    let text = theme.resolve(Token::TextPrimary);
    let accent = theme.resolve(Token::Accent);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let fill = Style::new().fg(text).bg(surface);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let (mode, query, step) = kh_state(tick);
    let header = Style::new()
        .fg(accent)
        .bg(raised)
        .with_attrs(Attrs::new().set_bold(true));
    cx.put_str(area.x, area.y, &" ".repeat(usize::from(area.width)), header);
    cx.put_str(
        area.x + 1,
        area.y,
        "Release console  ·  lunatui v0.1.13",
        header,
    );
    let shown: Vec<(&str, &str)> = KH_CHECKS
        .iter()
        .copied()
        .filter(|(name, _)| query.is_empty() || name.contains(query.as_str()))
        .collect();
    let panes = Rect::new(area.x + 2, area.y + 2, area.width.saturating_sub(4), 15);
    kh_log(
        &mut cx,
        Rect::new(
            area.x + 2,
            area.y + 17,
            area.width.saturating_sub(4),
            area.height.saturating_sub(19),
        ),
        tick,
        [fill, dim],
        [accent, raised],
    );
    kh_panes(
        &mut cx,
        panes,
        &shown,
        (mode, &query, step),
        [fill, dim],
        [accent, raised],
    );
    if mode == 2 {
        kh_confirm(&mut cx, area, [fill, dim], accent);
    }
    let bar = Rect::new(
        area.x,
        area.y + area.height.saturating_sub(1),
        area.width,
        1,
    );
    cx.put_str(
        bar.x,
        bar.y,
        &" ".repeat(usize::from(bar.width)),
        Style::new().fg(text).bg(raised),
    );
    KeyHintBar::new(&kh_keymap(mode)).gap(3).render(
        Rect::new(bar.x + 1, bar.y, bar.width.saturating_sub(2), 1),
        &mut cx,
    );
}

fn kh_panes(
    cx: &mut RenderCx<'_>,
    area: Rect,
    shown: &[(&str, &str)],
    state: (usize, &str, usize),
    styles: [Style; 2],
    colors: [Color; 2],
) {
    let (mode, query, step) = state;
    let [fill, dim] = styles;
    let [accent, raised] = colors;
    let left = Rect::new(area.x, area.y, 46, area.height);
    let right = Rect::new(
        left.x + left.width + 1,
        area.y,
        area.width.saturating_sub(left.width + 1),
        area.height,
    );
    let title = if mode == 1 {
        format!(" Checks  /{query}▏ ")
    } else {
        String::from(" Checks ")
    };
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(title)
        .style(fill)
        .border_style(dim)
        .title_style(Style::new().fg(accent).bg(fill.bg))
        .render(left, cx);
    let selected = if shown.is_empty() {
        0
    } else {
        step % shown.len()
    };
    for (row, (name, took)) in (0_u16..).zip(shown) {
        let y = left.y + 2 + row;
        let picked = usize::from(row) == selected && mode != 1;
        let style = if picked {
            Style::new().fg(fill.fg).bg(raised)
        } else {
            fill
        };
        cx.put_str(
            left.x + 1,
            y,
            &" ".repeat(usize::from(left.width - 2)),
            style,
        );
        cx.put_str(left.x + 2, y, &format!("✔ {name:<14}{took:>20}"), style);
    }
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Release notes ")
        .style(fill)
        .border_style(dim)
        .title_style(Style::new().fg(accent).bg(fill.bg))
        .render(right, cx);
    let (name, took) = shown.get(selected).copied().unwrap_or(("none", ""));
    for (row, line) in (0_u16..).zip([
        format!("check      {name}"),
        format!("duration   {took}"),
        String::from("branch     main"),
        String::from("head       9c2b269"),
        String::new(),
        String::from("Changes since v0.1.12"),
        String::from("  • ToastStack, Canvas, SplitTree widgets"),
        String::from("  • Breadcrumbs and TabsDrag"),
        String::from("  • No-feature build fixed"),
    ]) {
        cx.put_str(
            right.x + 2,
            right.y + 2 + row,
            clip_display_prefix(&line, right.width.saturating_sub(4)),
            if row == 5 { fill.fg(accent) } else { fill },
        );
    }
}

fn kh_log(cx: &mut RenderCx<'_>, area: Rect, tick: u32, styles: [Style; 2], colors: [Color; 2]) {
    let [fill, dim] = styles;
    let [accent, _] = colors;
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Run log ")
        .style(fill)
        .border_style(dim)
        .title_style(Style::new().fg(accent).bg(fill.bg))
        .render(area, cx);
    let steps = [
        "checkout main",
        "cargo fmt --check",
        "cargo clippy -D warnings",
        "cargo test --locked",
        "cargo deny check",
        "cargo xtask bench",
        "build release artifacts",
        "sign checksums",
    ];
    let rows = usize::from(area.height.saturating_sub(2));
    let count = usize::try_from(tick / 6).unwrap_or(0) + 1;
    let start = count.saturating_sub(rows);
    for (row, index) in (0_u16..).zip(start..count) {
        let step = steps[index % steps.len()];
        let line = format!(
            "12:{:02}:{:02}  ok   {step}",
            (index * 7 / 60) % 60,
            index * 7 % 60
        );
        cx.put_str(
            area.x + 2,
            area.y + 1 + row,
            clip_display_prefix(&line, area.width.saturating_sub(4)),
            if index + 1 == count { fill } else { dim },
        );
    }
}

fn kh_confirm(cx: &mut RenderCx<'_>, area: Rect, styles: [Style; 2], accent: Color) {
    let [fill, dim] = styles;
    let modal = Rect::new(
        area.x + area.width / 2 - 22,
        area.y + area.height / 2 - 3,
        44,
        6,
    );
    for row in 0..modal.height {
        cx.put_str(modal.x, modal.y + row, &" ".repeat(44), fill);
    }
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Publish ")
        .style(fill)
        .border_style(Style::new().fg(accent).bg(fill.bg))
        .title_style(Style::new().fg(accent).bg(fill.bg))
        .render(modal, cx);
    cx.put_str(
        modal.x + 2,
        modal.y + 2,
        "Tag and publish lunatui v0.1.13?",
        fill,
    );
    cx.put_str(
        modal.x + 2,
        modal.y + 3,
        "All eight checks are passing.",
        dim,
    );
}

fn key_hints_baseline(buffer: &mut Buffer, area: RatRect, _theme: &Theme, tick: u32) {
    if let Some(case) = CASES.iter().find(|case| case.name == "key-hints") {
        (case.ratatui)(buffer, area, tick);
    }
}

const LS_TITLES: [(&str, &str); 28] = [
    ("Record the MarkdownView demo at 2x", "platform"),
    ("Add a key hint bar widget", "widgets"),
    ("Add TabsDrag for reordering tabs", "widgets"),
    ("Add SplitTree for resizable split panes", "widgets"),
    ("Clear table rows below the last data row", "widgets"),
    ("Add the Breadcrumbs widget", "widgets"),
    ("Add the Canvas widget", "widgets"),
    ("Add the ToastStack widget", "widgets"),
    ("Add the Chart widget", "widgets"),
    ("Expose the glyph a Spinner draws", "widgets"),
    ("Add left and right StatusBar segments", "widgets"),
    ("Build lunatui-widgets with no features", "build"),
    ("Rename the X10 mouse bytes binding", "term"),
    ("Use epsilon checks for animation floats", "runtime"),
    ("Remove redundant deploy board empty check", "examples"),
    ("Add a runnable Kitty Image demo", "image"),
    ("Keep the background under a focused Image ring", "image"),
    ("Add the MarkdownView widget", "widgets"),
    ("Make the Markdown parser linear-time", "text"),
    ("Add a Markdown parser to lunatui-text", "text"),
    ("Add the BarChart widget", "widgets"),
    ("Add named spinner presets", "widgets"),
    ("Guard the site gallery against painted frames", "site"),
    ("Release v0.1.12", "release"),
    ("Add Layout::split_wrapped", "layout"),
    ("Add Layout::limits and aspect_fit", "layout"),
    ("Style Table headers and cells", "widgets"),
    ("Add boxed Tabs", "widgets"),
];

fn ls_status(index: usize, tick: u32) -> (&'static str, Token, &'static str) {
    let flipped = index == 3 && tick >= 80;
    match (index % 4, flipped) {
        (0 | 3, false) => ("●", Token::Ok, "open"),
        (1, _) | (_, true) => ("✔", Token::Accent, "merged"),
        (2, _) => ("✖", Token::Fail, "closed"),
        _ => ("◐", Token::Warn, "draft"),
    }
}

fn ls_rows(tick: u32) -> Vec<(String, usize)> {
    let fresh = usize::from(tick >= 120);
    let mut rows = Vec::new();
    if fresh == 1 {
        rows.push((String::from("Add a toast for failed uploads"), 28));
    }
    for (index, (title, _)) in LS_TITLES.iter().enumerate() {
        rows.push(((*title).to_owned(), index));
    }
    rows
}

fn list_icons_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let raised = theme.resolve(Token::Surface1);
    let text = theme.resolve(Token::TextPrimary);
    let accent = theme.resolve(Token::Accent);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let fill = Style::new().fg(text).bg(surface);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let head = Style::new()
        .fg(accent)
        .bg(raised)
        .with_attrs(Attrs::new().set_bold(true));
    cx.put_str(area.x, area.y, &" ".repeat(usize::from(area.width)), head);
    cx.put_str(
        area.x + 1,
        area.y,
        "Pull requests  ·  moonbase2090/lunatui",
        head,
    );
    let rows = ls_rows(tick);
    let left = Rect::new(area.x + 2, area.y + 2, 76, area.height.saturating_sub(5));
    let right = Rect::new(
        left.x + left.width + 1,
        left.y,
        area.width.saturating_sub(left.width + 5),
        left.height,
    );
    ls_list(&mut cx, left, &rows, tick, [fill, dim], [accent, raised]);
    ls_detail(&mut cx, right, &rows, tick, [fill, dim], accent);
    StatusBar::new(" j/k move  ●  Enter open  ●  m merge  ●  q quit")
        .style(Style::new().fg(text).bg(raised))
        .render(
            Rect::new(
                area.x,
                area.y + area.height.saturating_sub(1),
                area.width,
                1,
            ),
            &mut cx,
        );
}

fn ls_selected(tick: u32, count: usize) -> usize {
    usize::try_from(tick / 9).unwrap_or(0) % count
}

fn ls_list(
    cx: &mut RenderCx<'_>,
    area: Rect,
    rows: &[(String, usize)],
    tick: u32,
    styles: [Style; 2],
    colors: [Color; 2],
) {
    let [fill, dim] = styles;
    let [accent, raised] = colors;
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Open and recent ")
        .style(fill)
        .border_style(dim)
        .title_style(Style::new().fg(accent).bg(fill.bg))
        .render(area, cx);
    let inner = Rect::new(
        area.x + 1,
        area.y + 1,
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    );
    let selected = ls_selected(tick, rows.len());
    let visible = usize::from(inner.height);
    let offset = selected
        .saturating_sub(visible.saturating_sub(4))
        .min(rows.len().saturating_sub(visible));
    let numbers: Vec<usize> = rows.iter().map(|(_, index)| *index).collect();
    let icons: Vec<(String, Style)> = numbers
        .iter()
        .map(|index| {
            let (glyph, token, _) = ls_status(*index, tick);
            ((*glyph).to_owned(), Style::new().fg(cx.token(token)))
        })
        .collect();
    let marks: Vec<Option<(String, Style)>> = rows
        .iter()
        .map(|(_, index)| {
            let label = LS_TITLES.get(*index).map_or("new", |(_, tag)| tag);
            Some((
                format!("#{}  {label}", 200_usize.saturating_sub(*index)),
                dim,
            ))
        })
        .collect();
    let source = VecSource::from_rows(rows.iter().map(|(title, _)| vec![title.clone()]).collect());
    List::new(source)
        .style(fill)
        .highlight_style(Style::new().fg(fill.fg).bg(raised))
        .highlight_symbol("▸", Style::new().fg(accent).bg(raised))
        .icons(icons)
        .marks(marks, 2)
        .offset(offset)
        .selected(Some(selected))
        .render(inner, cx);
}

fn ls_detail(
    cx: &mut RenderCx<'_>,
    area: Rect,
    rows: &[(String, usize)],
    tick: u32,
    styles: [Style; 2],
    accent: Color,
) {
    let [fill, dim] = styles;
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Details ")
        .style(fill)
        .border_style(dim)
        .title_style(Style::new().fg(accent).bg(fill.bg))
        .render(area, cx);
    let selected = ls_selected(tick, rows.len());
    let (title, index) = &rows[selected];
    let (_, _, state) = ls_status(*index, tick);
    let tag = LS_TITLES.get(*index).map_or("new", |(_, tag)| tag);
    for (row, line) in (0_u16..).zip([
        title.clone(),
        String::new(),
        format!("state     {state}"),
        format!("number    #{}", 200_usize.saturating_sub(*index)),
        format!("area      {tag}"),
        String::from("author    Moonbase 2090"),
        String::from("checks    12 of 12 passing"),
        String::from("review    approved"),
    ]) {
        let style = if row == 0 { fill.fg(accent) } else { fill };
        cx.put_str(
            area.x + 2,
            area.y + 1 + row,
            clip_display_prefix(&line, area.width.saturating_sub(4)),
            style,
        );
    }
}

fn list_icons_baseline(_buffer: &mut Buffer, _area: RatRect, _theme: &Theme, _tick: u32) {}

const TG_ITEMS: [(&str, &str, &str, bool); 8] = [
    (
        "Dark mode",
        "Use the dark theme everywhere",
        "dark_mode",
        true,
    ),
    (
        "Reduce motion",
        "Skip transitions and animations",
        "reduce_motion",
        false,
    ),
    (
        "Compact layout",
        "Tighter rows in tables and lists",
        "compact",
        false,
    ),
    (
        "Line numbers",
        "Show line numbers in the editor",
        "line_numbers",
        true,
    ),
    (
        "Desktop alerts",
        "Notify when a build finishes",
        "desktop_alerts",
        true,
    ),
    (
        "Alert sound",
        "Play a sound with each alert",
        "alert_sound",
        false,
    ),
    (
        "Share usage data",
        "Send anonymous feature counts",
        "usage_data",
        false,
    ),
    (
        "Crash reports",
        "Send a report when the app fails",
        "crash_reports",
        true,
    ),
];

fn tg_state(tick: u32) -> ([bool; 8], usize) {
    let mut on = [false; 8];
    for (slot, item) in on.iter_mut().zip(TG_ITEMS) {
        *slot = item.3;
    }
    let focus = usize::try_from(tick / 16).unwrap_or(0) % 8;
    for pass in 0..2_u32 {
        for index in 0..8_u32 {
            let at = pass * 128 + index * 16 + 8;
            let skipped = pass == 1 && index % 2 == 1;
            if tick >= at && !skipped {
                let slot = usize::try_from(index).unwrap_or(0);
                on[slot] = !on[slot];
            }
        }
    }
    (on, focus)
}

fn tg_extra(cx: &mut RenderCx<'_>, config: Rect, styles: [Style; 2], accent: Color) {
    let [fill, dim] = styles;
    for (row, line) in (0_u16..).zip([
        "",
        "[editor]",
        "tab_width       = 4",
        "font            = \"JetBrains Mono\"",
        "font_size       = 14",
        "",
        "[updates]",
        "channel         = \"stable\"",
    ]) {
        let y = config.y + 10 + row;
        if y + 1 >= config.y + config.height {
            break;
        }
        let style = if line.starts_with('[') {
            fill.fg(accent)
        } else {
            dim
        };
        cx.put_str(config.x + 2, y, line, style);
    }
}

fn toggle_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let raised = theme.resolve(Token::Surface1);
    let text = theme.resolve(Token::TextPrimary);
    let accent = theme.resolve(Token::Accent);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let fill = Style::new().fg(text).bg(surface);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let head = Style::new()
        .fg(accent)
        .bg(raised)
        .with_attrs(Attrs::new().set_bold(true));
    cx.put_str(area.x, area.y, &" ".repeat(usize::from(area.width)), head);
    cx.put_str(area.x + 1, area.y, "Settings", head);
    let (on, focus) = tg_state(tick);
    let width = (area.width.saturating_sub(5)) / 2;
    let left = Rect::new(area.x + 2, area.y + 2, width, 14);
    let right = Rect::new(left.x + width + 1, left.y, width, 14);
    for (title, pane, first) in [
        (" General ", left, 0_usize),
        (" Notifications and privacy ", right, 4),
    ] {
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(title)
            .style(fill)
            .border_style(dim)
            .title_style(Style::new().fg(accent).bg(surface))
            .render(pane, &mut cx);
        for row in 0..4_u16 {
            let index = first + usize::from(row);
            let y = pane.y + 1 + row * 3;
            let state = if index == focus {
                WidgetState::Focused
            } else {
                WidgetState::Normal
            };
            Toggle::new(on[index])
                .label(TG_ITEMS[index].0)
                .state(state)
                .render(
                    Rect::new(pane.x + 2, y, pane.width.saturating_sub(4), 1),
                    &mut cx,
                );
            cx.put_str(pane.x + 2 + 5, y + 1, TG_ITEMS[index].1, dim);
        }
    }
    let config = Rect::new(
        area.x + 2,
        left.y + 15,
        area.width.saturating_sub(4),
        area.height.saturating_sub(left.y + 18),
    );
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" ~/.config/lunatui/settings.toml ")
        .style(fill)
        .border_style(dim)
        .title_style(Style::new().fg(accent).bg(surface))
        .render(config, &mut cx);
    cx.put_str(config.x + 2, config.y + 1, "[app]", fill.fg(accent));
    for (row, (item, value)) in (0_u16..).zip(TG_ITEMS.iter().zip(on)) {
        if row + 4 >= config.height {
            break;
        }
        let style = if usize::from(row) == focus { fill } else { dim };
        cx.put_str(
            config.x + 2,
            config.y + 2 + row,
            &format!("{:<16}= {value}", item.2),
            style,
        );
    }
    tg_extra(&mut cx, config, [fill, dim], accent);
    StatusBar::new(" Tab next setting  ●  Space toggle  ●  Esc close")
        .style(Style::new().fg(text).bg(raised))
        .render(
            Rect::new(
                area.x,
                area.y + area.height.saturating_sub(1),
                area.width,
                1,
            ),
            &mut cx,
        );
}

fn code_view_baseline(buffer: &mut Buffer, area: RatRect, _theme: &Theme, tick: u32) {
    if let Some(case) = CASES.iter().find(|case| case.name == "code-view") {
        (case.ratatui)(buffer, area, tick);
    }
}

fn button_baseline(buffer: &mut Buffer, area: RatRect, _theme: &Theme, tick: u32) {
    if let Some(case) = CASES.iter().find(|case| case.name == "button") {
        (case.ratatui)(buffer, area, tick);
    }
}

fn table_sorted_baseline(buffer: &mut Buffer, area: RatRect, _theme: &Theme, tick: u32) {
    if let Some(case) = CASES.iter().find(|case| case.name == "table-sorted") {
        (case.ratatui)(buffer, area, tick);
    }
}

fn dialog_baseline(buffer: &mut Buffer, area: RatRect, _theme: &Theme, tick: u32) {
    if let Some(case) = CASES.iter().find(|case| case.name == "dialog") {
        (case.ratatui)(buffer, area, tick);
    }
}

fn popover_baseline(buffer: &mut Buffer, area: RatRect, _theme: &Theme, tick: u32) {
    if let Some(case) = CASES.iter().find(|case| case.name == "popover") {
        (case.ratatui)(buffer, area, tick);
    }
}

fn tooltip_baseline(buffer: &mut Buffer, area: RatRect, _theme: &Theme, tick: u32) {
    if let Some(case) = CASES.iter().find(|case| case.name == "tooltip") {
        (case.ratatui)(buffer, area, tick);
    }
}

fn toggle_baseline(buffer: &mut Buffer, area: RatRect, _theme: &Theme, tick: u32) {
    if let Some(case) = CASES.iter().find(|case| case.name == "toggle") {
        (case.ratatui)(buffer, area, tick);
    }
}

const RD_THEME: [&str; 3] = ["Light", "Dark", "Match system"];
const RD_SIZES: [&str; 4] = ["Small", "Medium", "Large", "Extra large"];
const RD_CHANNELS: [&str; 3] = ["Stable", "Beta", "Nightly (unavailable)"];

fn rd_groups() -> [RadioGroup; 3] {
    [
        RadioGroup::new(RD_THEME),
        RadioGroup::new(RD_SIZES).flow(RadioFlow::Horizontal).gap(3),
        RadioGroup::new(RD_CHANNELS).disabled_options([2]),
    ]
}

fn rd_state(tick: u32) -> ([Option<usize>; 3], Option<usize>) {
    let groups = rd_groups();
    let mut selected = [Some(2), Some(1), Some(0)];
    let mut focus = None;
    for event in 0..24_u32 {
        let at = event * 10 + 8;
        if tick < at {
            break;
        }
        let which = usize::try_from(event % 3).unwrap_or(0);
        let forward = event % 7 != 4;
        selected[which] = groups[which].step(selected[which], forward);
        focus = Some(which);
    }
    (selected, focus)
}

fn radio_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let raised = theme.resolve(Token::Surface1);
    let text = theme.resolve(Token::TextPrimary);
    let accent = theme.resolve(Token::Accent);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let fill = Style::new().fg(text).bg(surface);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let head = Style::new()
        .fg(accent)
        .bg(raised)
        .with_attrs(Attrs::new().set_bold(true));
    cx.put_str(area.x, area.y, &" ".repeat(usize::from(area.width)), head);
    cx.put_str(area.x + 1, area.y, "Preferences", head);
    let (selected, focus) = rd_state(tick);
    let left = Rect::new(area.x + 2, area.y + 2, 60, 24);
    rd_prefs(&mut cx, left, selected, focus, [fill, dim], accent);
    let right = Rect::new(
        left.x + left.width + 1,
        left.y,
        area.width.saturating_sub(left.width + 5),
        24,
    );
    rd_preview(&mut cx, right, selected, [fill, dim], accent);
    StatusBar::new(" Tab next group  ●  Up/Down change option  ●  Esc close")
        .style(Style::new().fg(text).bg(raised))
        .render(
            Rect::new(
                area.x,
                area.y + area.height.saturating_sub(1),
                area.width,
                1,
            ),
            &mut cx,
        );
}

fn rd_prefs(
    cx: &mut RenderCx<'_>,
    area: Rect,
    selected: [Option<usize>; 3],
    focus: Option<usize>,
    styles: [Style; 2],
    accent: Color,
) {
    let [fill, dim] = styles;
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Appearance and updates ")
        .style(fill)
        .border_style(dim)
        .title_style(Style::new().fg(accent).bg(fill.bg))
        .render(area, cx);
    let groups = rd_groups();
    let titles = ["Theme", "Text size", "Update channel"];
    let notes = [
        "Applies to every window",
        "Used by the editor and the terminal",
        "Nightly builds are not available on this plan",
    ];
    let tops = [area.y + 2, area.y + 10, area.y + 16];
    for (index, group) in groups.into_iter().enumerate() {
        let y = tops[index];
        let title_style = if focus == Some(index) {
            fill.fg(accent)
        } else {
            fill
        };
        cx.put_str(area.x + 3, y, titles[index], title_style);
        cx.put_str(area.x + 3, y + 1, notes[index], dim);
        let bounds = Rect::new(area.x + 3, y + 3, area.width.saturating_sub(6), 4);
        group
            .select(selected[index])
            .focus(if focus == Some(index) {
                selected[index]
            } else {
                None
            })
            .render(bounds, cx);
    }
}

fn rd_pick(selected: [Option<usize>; 3], group: usize, names: &[&'static str]) -> &'static str {
    selected[group]
        .and_then(|index| names.get(index).copied())
        .unwrap_or("none")
}

fn rd_preview(
    cx: &mut RenderCx<'_>,
    area: Rect,
    selected: [Option<usize>; 3],
    styles: [Style; 2],
    accent: Color,
) {
    let [fill, dim] = styles;
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Preview ")
        .style(fill)
        .border_style(dim)
        .title_style(Style::new().fg(accent).bg(fill.bg))
        .render(area, cx);
    let lines = [
        format!("theme     {}", rd_pick(selected, 0, &RD_THEME)),
        format!("size      {}", rd_pick(selected, 1, &RD_SIZES)),
        format!("channel   {}", rd_pick(selected, 2, &RD_CHANNELS)),
        String::new(),
        String::from("use std::collections::HashMap;"),
        String::new(),
        String::from("fn main() {"),
        String::from("    let greeting = \"hello\";"),
        String::from("    let mut counts: HashMap<char, u32> = HashMap::new();"),
        String::from("    for letter in greeting.chars() {"),
        String::from("        *counts.entry(letter).or_insert(0) += 1;"),
        String::from("    }"),
        String::new(),
        String::from("    let mut pairs: Vec<_> = counts.into_iter().collect();"),
        String::from("    pairs.sort();"),
        String::from("    println!(\"{pairs:?}\");"),
        String::from("}"),
    ];
    for (row, line) in (0_u16..).zip(lines) {
        let style = if row < 3 { fill } else { dim };
        cx.put_str(
            area.x + 2,
            area.y + 2 + row,
            clip_display_prefix(&line, area.width.saturating_sub(4)),
            style,
        );
    }
}

fn radio_baseline(buffer: &mut Buffer, area: RatRect, _theme: &Theme, tick: u32) {
    if let Some(case) = CASES.iter().find(|case| case.name == "radio-group") {
        (case.ratatui)(buffer, area, tick);
    }
}

struct FmState {
    name: String,
    email: String,
    password: usize,
    notes: String,
    plan: usize,
    news: bool,
    focus: Option<usize>,
    checked: bool,
    done: bool,
}

fn fm_prefix(text: &str, count: u32) -> String {
    text.chars()
        .take(usize::try_from(count).unwrap_or(0))
        .collect()
}

fn fm_state(tick: u32) -> FmState {
    let password = if tick >= 210 {
        (6 + (tick - 210) / 3).min(9)
    } else {
        ((tick.saturating_sub(52)) / 4).min(6)
    };
    let focus = match tick {
        0..40 => Some(0),
        40..50 | 160..205 => Some(1),
        50..80 | 205..232 => Some(2),
        80..100 => Some(3),
        100..120 => Some(4),
        _ => Some(5),
    };
    FmState {
        name: fm_prefix("Ada Lovelace", tick.saturating_sub(4) / 3),
        email: fm_prefix("ada@example.org", tick.saturating_sub(168) / 2),
        password: usize::try_from(password).unwrap_or(0),
        notes: fm_prefix("Beta tester", tick.saturating_sub(124) / 2),
        plan: usize::from(tick >= 92),
        news: tick >= 110,
        focus,
        checked: tick >= 160,
        done: tick >= 236,
    }
}

fn fm_form(state: &FmState) -> Form {
    let mut email = FormField::new("Email").required().hint("Used to sign in");
    let mut password = FormField::new("Password")
        .required()
        .hint("At least 8 characters");
    if state.checked && state.email.is_empty() {
        email = email.error("Required");
    }
    if state.checked && state.password < 8 {
        password = password.error("Use at least 8 characters");
    }
    Form::new([
        FormField::new("Name")
            .required()
            .hint("As it appears on invoices"),
        email,
        password,
        FormField::new("Plan"),
        FormField::new("Updates"),
        FormField::new("Notes").hint("Optional"),
    ])
    .focus(state.focus)
}

fn fm_inputs(cx: &mut RenderCx<'_>, areas: &[FieldArea], state: &FmState, colors: [Color; 3]) {
    let [text, accent, raised] = colors;
    for area in areas {
        let focused = state.focus == Some(area.index);
        let field_bg = if focused {
            raised
        } else {
            cx.token(Token::Surface0)
        };
        let field = Style::new().fg(text).bg(field_bg);
        let input = area.input;
        match area.index {
            0 | 1 | 5 => {
                let value = match area.index {
                    0 => state.name.clone(),
                    1 => state.email.clone(),
                    _ => state.notes.clone(),
                };
                let cursor = value.len();
                TextInput::new(value, cursor)
                    .style(field)
                    .field_style(field)
                    .cursor_style(Style::new().fg(accent).bg(field_bg))
                    .padding(1)
                    .show_cursor(focused)
                    .render(input, cx);
            }
            2 => {
                let dots = "•".repeat(state.password);
                let cursor = dots.len();
                TextInput::new(dots, cursor)
                    .style(field)
                    .field_style(field)
                    .cursor_style(Style::new().fg(accent).bg(field_bg))
                    .padding(1)
                    .show_cursor(focused)
                    .render(input, cx);
            }
            3 => RadioGroup::new(["Free", "Pro", "Team"])
                .select(Some(state.plan))
                .flow(RadioFlow::Horizontal)
                .gap(3)
                .focus(focused.then_some(state.plan))
                .render(input, cx),
            _ => Toggle::new(state.news)
                .label("Product news by email")
                .state(if focused {
                    WidgetState::Focused
                } else {
                    WidgetState::Normal
                })
                .render(input, cx),
        }
    }
}

fn form_native(grid: &mut Grid, clusters: &mut ClusterTable, theme: &Theme, area: Rect, tick: u32) {
    let surface = theme.resolve(Token::Surface0);
    let raised = theme.resolve(Token::Surface1);
    let text = theme.resolve(Token::TextPrimary);
    let accent = theme.resolve(Token::Accent);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let fill = Style::new().fg(text).bg(surface);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let head = Style::new()
        .fg(accent)
        .bg(raised)
        .with_attrs(Attrs::new().set_bold(true));
    cx.put_str(area.x, area.y, &" ".repeat(usize::from(area.width)), head);
    cx.put_str(area.x + 1, area.y, "Lunatui  ·  Create your account", head);
    let state = fm_state(tick);
    let panel = Rect::new(area.x + 2, area.y + 2, 70, 28);
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Account details ")
        .style(fill)
        .border_style(dim)
        .title_style(Style::new().fg(accent).bg(surface))
        .render(panel, &mut cx);
    let inner = Rect::new(panel.x + 2, panel.y + 2, panel.width - 4, panel.height - 3);
    let form = fm_form(&state);
    let areas = form.layout(inner);
    form.render(inner, &mut cx);
    fm_inputs(&mut cx, &areas, &state, [text, accent, raised]);
    let side = Rect::new(
        panel.x + panel.width + 1,
        panel.y,
        area.width.saturating_sub(panel.width + 5),
        14,
    );
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Review ")
        .style(fill)
        .border_style(dim)
        .title_style(Style::new().fg(accent).bg(surface))
        .render(side, &mut cx);
    let plans = ["Free", "Pro", "Team"];
    for (row, line) in (0_u16..).zip([
        format!("name      {}", state.name),
        format!("email     {}", state.email),
        format!("password  {} characters", state.password),
        format!("plan      {}", plans[state.plan.min(2)]),
        format!("updates   {}", if state.news { "on" } else { "off" }),
        format!("notes     {}", state.notes),
    ]) {
        cx.put_str(
            side.x + 2,
            side.y + 2 + row,
            clip_display_prefix(&line, side.width.saturating_sub(4)),
            fill,
        );
    }
    let attention = usize::from(state.email.is_empty()) + usize::from(state.password < 8);
    let status = if state.done {
        String::from(" Account created  ●  welcome email sent")
    } else if state.checked && attention > 0 {
        format!(" {attention} field(s) need attention  ●  Tab next  ●  Enter submit")
    } else {
        String::from(" Tab next field  ●  Enter submit  ●  Esc cancel")
    };
    StatusBar::new(status)
        .style(Style::new().fg(text).bg(raised))
        .render(
            Rect::new(
                area.x,
                area.y + area.height.saturating_sub(1),
                area.width,
                1,
            ),
            &mut cx,
        );
}

fn form_baseline(_buffer: &mut Buffer, _area: RatRect, _theme: &Theme, _tick: u32) {}

const SL_CHANNELS: [(&str, f64); 8] = [
    ("Master", 80.0),
    ("Music", 64.0),
    ("Voice", 72.0),
    ("Effects", 45.0),
    ("Ambience", 30.0),
    ("Alerts", 55.0),
    ("Reverb", 22.0),
    ("Monitor", 68.0),
];

const SL_BANDS: [(&str, f64, f64, f64, f64); 4] = [
    ("Low cut", 20.0, 300.0, 40.0, 180.0),
    ("Mid band", 300.0, 4000.0, 600.0, 2400.0),
    ("High band", 4000.0, 16000.0, 5200.0, 12800.0),
    ("Air band", 8000.0, 20000.0, 9500.0, 18000.0),
];

fn sl_wave(tick: u32, offset: u32, low: f64, high: f64) -> f64 {
    let t = tri(tick + offset, 90, 0, 1000);
    low + (high - low) * f64::from(u32::try_from(t).unwrap_or(0)) / 1000.0
}

fn sl_values(tick: u32) -> ([f64; 8], usize) {
    let focus = usize::try_from(tick / 40).unwrap_or(0) % SL_CHANNELS.len();
    let mut values = [0.0; 8];
    for (index, (slot, (_, base))) in values.iter_mut().zip(SL_CHANNELS).enumerate() {
        *slot = if index == focus {
            sl_wave(tick, 20, (base - 28.0).max(5.0), (base + 18.0).min(100.0)).round()
        } else {
            base
        };
    }
    (values, focus)
}

fn slider_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let raised = theme.resolve(Token::Surface1);
    let text = theme.resolve(Token::TextPrimary);
    let accent = theme.resolve(Token::Accent);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let fill = Style::new().fg(text).bg(surface);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let head = Style::new()
        .fg(accent)
        .bg(raised)
        .with_attrs(Attrs::new().set_bold(true));
    cx.put_str(area.x, area.y, &" ".repeat(usize::from(area.width)), head);
    cx.put_str(area.x + 1, area.y, "Studio mixer  ·  session 14", head);
    let (values, focus) = sl_values(tick);
    let left = Rect::new(area.x + 2, area.y + 2, 62, 26);
    let right = Rect::new(
        left.x + left.width + 1,
        left.y,
        area.width.saturating_sub(left.width + 5),
        26,
    );
    for (title, pane) in [(" Channels ", left), (" Equalizer ranges (Hz) ", right)] {
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(title)
            .style(fill)
            .border_style(dim)
            .title_style(Style::new().fg(accent).bg(surface))
            .render(pane, &mut cx);
    }
    sl_channels(&mut cx, left, values, focus);
    sl_bands(&mut cx, right, tick, [fill, dim]);
    let percent = values[0] * values[1] / 100.0;
    let level = (0..=100_u8)
        .find(|n| f64::from(*n) >= percent)
        .map_or(1.0, |n| f32::from(n) / 100.0);
    let meter = Rect::new(area.x + 2, left.y + 27, area.width.saturating_sub(4), 3);
    Gauge::new(level)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(" Output level ")
                .style(fill)
                .border_style(dim)
                .title_style(Style::new().fg(accent).bg(surface)),
        )
        .gauge_style(Style::new().fg(accent).bg(surface))
        .render(meter, &mut cx);
    StatusBar::new(" Tab next channel  ●  Left/Right adjust  ●  Shift+Left/Right big step")
        .style(Style::new().fg(text).bg(raised))
        .render(
            Rect::new(
                area.x,
                area.y + area.height.saturating_sub(1),
                area.width,
                1,
            ),
            &mut cx,
        );
}

fn sl_channels(cx: &mut RenderCx<'_>, area: Rect, values: [f64; 8], focus: usize) {
    for (index, ((name, _), value)) in SL_CHANNELS.iter().zip(values).enumerate() {
        let state = if index == focus {
            WidgetState::Focused
        } else {
            WidgetState::Normal
        };
        Slider::new(0.0, 100.0)
            .value(value)
            .step(1.0)
            .label(format!("{name:<9}"))
            .show_value(true, 0)
            .knob(index == focus)
            .state(state)
            .render(
                Rect::new(
                    area.x + 3,
                    area.y + 2 + u16::try_from(index).unwrap_or(0) * 3,
                    area.width - 6,
                    1,
                ),
                cx,
            );
    }
}

fn sl_bands(cx: &mut RenderCx<'_>, area: Rect, tick: u32, styles: [Style; 2]) {
    let [fill, dim] = styles;
    for (index, (name, min, max, low, high)) in SL_BANDS.iter().enumerate() {
        let drift =
            sl_wave(tick, u32::try_from(index).unwrap_or(0) * 30, -0.08, 0.08) * (max - min);
        let (first, second) = (low + drift, high - drift);
        let y = area.y + 2 + u16::try_from(index).unwrap_or(0) * 5;
        cx.put_str(area.x + 3, y, name, fill);
        Slider::new(*min, *max)
            .range(first, second)
            .knob(true)
            .show_value(true, 0)
            .render(Rect::new(area.x + 3, y + 1, area.width - 6, 1), cx);
        cx.put_str(
            area.x + 3,
            y + 2,
            &format!("width {:.0} Hz", second - first),
            dim,
        );
    }
}

fn slider_baseline(_buffer: &mut Buffer, _area: RatRect, _theme: &Theme, _tick: u32) {}

type NumberField = (
    &'static str,
    &'static str,
    f64,
    f64,
    f64,
    f64,
    u8,
    &'static str,
    i32,
);

const NM_FIELDS: [NumberField; 6] = [
    (
        "Width",
        "Multiple of 16 pixels",
        1920.0,
        320.0,
        7680.0,
        16.0,
        0,
        " px",
        10,
    ),
    (
        "Height",
        "Multiple of 8 pixels",
        1080.0,
        240.0,
        4320.0,
        8.0,
        0,
        " px",
        10,
    ),
    (
        "Frame rate",
        "Frames per second",
        24.0,
        1.0,
        120.0,
        0.01,
        2,
        " fps",
        0,
    ),
    (
        "Quality",
        "Percent, 0 to 100",
        70.0,
        0.0,
        100.0,
        1.0,
        0,
        "%",
        3,
    ),
    (
        "Bitrate",
        "Megabits per second",
        8.0,
        0.5,
        80.0,
        0.5,
        1,
        " Mbps",
        4,
    ),
    (
        "Copies",
        "Files written per export",
        1.0,
        1.0,
        99.0,
        1.0,
        0,
        "",
        1,
    ),
];

const NM_EVENTS: [u32; 4] = [15, 20, 25, 30];
const NM_TYPING: [(u32, &str); 6] = [
    (12, ""),
    (18, "2"),
    (24, "23"),
    (30, "23."),
    (34, "23.9"),
    (38, "23.97"),
];

fn nm_input(index: usize, value: f64) -> NumberInput<'static> {
    let (_, _, _, min, max, step, decimals, suffix, _) = NM_FIELDS[index];
    NumberInput::new(value)
        .range(min, max)
        .step(step)
        .decimals(decimals)
        .suffix(suffix)
}

fn nm_value(index: usize, tick: u32) -> f64 {
    let (_, _, base, _, _, _, _, _, per_event) = NM_FIELDS[index];
    let local = tick.saturating_sub(u32::try_from(index).unwrap_or(0) * 40);
    if index == 2 {
        return if local > 38 { 23.97 } else { base };
    }
    NM_EVENTS
        .iter()
        .filter(|event| local >= **event)
        .fold(base, |value, _| nm_input(index, value).nudge(per_event))
}

fn nm_edit(tick: u32) -> Option<(&'static str, usize)> {
    let local = tick.checked_sub(80).filter(|local| *local <= 38)?;
    NM_TYPING
        .iter()
        .rev()
        .find(|(at, _)| local >= *at)
        .map(|(_, text)| (*text, text.len()))
}

fn number_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let raised = theme.resolve(Token::Surface1);
    let text = theme.resolve(Token::TextPrimary);
    let accent = theme.resolve(Token::Accent);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let fill = Style::new().fg(text).bg(surface);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let head = Style::new()
        .fg(accent)
        .bg(raised)
        .with_attrs(Attrs::new().set_bold(true));
    cx.put_str(area.x, area.y, &" ".repeat(usize::from(area.width)), head);
    cx.put_str(area.x + 1, area.y, "Render queue  ·  export settings", head);
    let left = Rect::new(area.x + 2, area.y + 2, 62, 30);
    let right = Rect::new(
        left.x + left.width + 1,
        left.y,
        area.width.saturating_sub(left.width + 5),
        30,
    );
    for (title, pane) in [(" Output ", left), (" Estimate ", right)] {
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(title)
            .style(fill)
            .border_style(dim)
            .title_style(Style::new().fg(accent).bg(surface))
            .render(pane, &mut cx);
    }
    let focus = usize::try_from(tick / 40).unwrap_or(5).min(5);
    for (index, (label, hint, ..)) in NM_FIELDS.iter().enumerate() {
        let y = left.y + 2 + u16::try_from(index).unwrap_or(0) * 4;
        let mut input = nm_input(index, nm_value(index, tick));
        if index == focus {
            input = input.state(WidgetState::Focused);
            if let (2, Some((typed, caret))) = (index, nm_edit(tick)) {
                input = input.edit(typed, caret);
            }
        }
        cx.put_str(left.x + 3, y, label, fill);
        input.render(Rect::new(left.x + 20, y, 30, 1), &mut cx);
        cx.put_str(left.x + 20, y + 1, hint, dim);
    }
    nm_estimate(&mut cx, right, tick, [fill, dim]);
    StatusBar::new(" ↑↓ field  ●  − + step  ●  type a value  ●  enter commit  ●  q quit")
        .style(Style::new().fg(text).bg(raised))
        .render(
            Rect::new(
                area.x,
                area.y + area.height.saturating_sub(1),
                area.width,
                1,
            ),
            &mut cx,
        );
}

fn nm_estimate(cx: &mut RenderCx<'_>, area: Rect, tick: u32, styles: [Style; 2]) {
    let [fill, dim] = styles;
    let value = |index| nm_value(index, tick);
    let size = value(4) * 120.0 / 8.0 * value(5);
    let rows = [
        ("Resolution", format!("{:.0} × {:.0}", value(0), value(1))),
        ("Frame rate", format!("{:.2} fps", value(2))),
        ("Quality", format!("{:.0}%", value(3))),
        ("Bitrate", format!("{:.1} Mbps", value(4))),
        ("Length", "2 min".to_owned()),
        ("Files", format!("{:.0}", value(5))),
        ("Size", format!("{size:.0} MB")),
    ];
    for (row, (label, text)) in (0_u16..).zip(rows) {
        cx.put_str(area.x + 2, area.y + 2 + row * 2, label, dim);
        cx.put_str(area.x + 15, area.y + 2 + row * 2, &text, fill);
    }
    let queue = ["intro.mov", "scene-02.mov", "scene-03.mov", "outro.mov"];
    cx.put_str(area.x + 2, area.y + 18, "Queue", dim);
    for (row, name) in (0_u16..).zip(queue) {
        cx.put_str(area.x + 2, area.y + 20 + row * 2, name, fill);
    }
}

fn number_baseline(_buffer: &mut Buffer, _area: RatRect, _theme: &Theme, _tick: u32) {}

const AC_SECTIONS: [(&str, &str, &[&str]); 7] = [
    (
        "Build",
        "3 settings",
        &[
            "profile   release",
            "target    aarch64-apple-darwin",
            "jobs      8",
        ],
    ),
    (
        "Environment",
        "4 variables",
        &[
            "RUST_LOG          info",
            "CARGO_TERM_COLOR  always",
            "TZ                UTC",
            "CI                true",
        ],
    ),
    (
        "Dependencies",
        "212 crates",
        &[
            "serde          1.0.219",
            "unicode-width  0.2.1",
            "bitflags       2.9.4",
            "smallvec       1.15.1",
        ],
    ),
    (
        "Tests",
        "3 suites",
        &[
            "unit         412 passed",
            "integration   96 passed",
            "doctests      31 passed",
        ],
    ),
    (
        "Deploy",
        "2 targets",
        &["channel  stable", "region   us-west-2"],
    ),
    (
        "Cache",
        "1.8 GB",
        &[
            "registry   shared",
            "target     per branch",
            "keep       14 days",
        ],
    ),
    (
        "Notifications",
        "2 channels",
        &["on failure  team chat", "on release  email digest"],
    ),
];

const AC_TARGETS: [[bool; 7]; 4] = [
    [true, false, false, false, false, false, false],
    [false, true, false, false, false, false, false],
    [false, true, true, false, false, false, false],
    [false, false, true, true, false, false, false],
];

fn ac_state(index: usize, tick: u32) -> (bool, f64) {
    let phase = usize::try_from(tick / 60).unwrap_or(3).min(3);
    let step = tick.saturating_sub(u32::try_from(phase).unwrap_or(0) * 60);
    let now = AC_TARGETS[phase][index];
    let before = AC_TARGETS[phase.saturating_sub(1)][index];
    let ramp = f64::from(step.saturating_sub(10).min(20)) / 20.0;
    let eased = ramp * ramp * (3.0 - 2.0 * ramp);
    let reveal = match (before, now) {
        (false, true) => eased,
        (true, false) => 1.0 - eased,
        (open, _) => f64::from(u8::from(open)),
    };
    (if step >= 10 { now } else { before }, reveal)
}

fn ac_accordion(tick: u32) -> Accordion {
    let phase = usize::try_from(tick / 60).unwrap_or(3).min(3);
    let sections = AC_SECTIONS
        .iter()
        .enumerate()
        .map(|(index, (title, badge, body))| {
            let (open, reveal) = ac_state(index, tick);
            AccordionSection::new(*title, u16::try_from(body.len()).unwrap_or(0))
                .open(open)
                .reveal(reveal)
                .badge(*badge)
        });
    Accordion::new(sections).focus(Some(phase)).gap(1)
}

fn accordion_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let raised = theme.resolve(Token::Surface1);
    let text = theme.resolve(Token::TextPrimary);
    let accent = theme.resolve(Token::Accent);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let fill = Style::new().fg(text).bg(surface);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let head = Style::new()
        .fg(accent)
        .bg(raised)
        .with_attrs(Attrs::new().set_bold(true));
    cx.put_str(area.x, area.y, &" ".repeat(usize::from(area.width)), head);
    cx.put_str(
        area.x + 1,
        area.y,
        "Pipeline settings  ·  release-aarch64",
        head,
    );
    let left = Rect::new(area.x + 2, area.y + 2, 60, 30);
    let right = Rect::new(
        left.x + left.width + 1,
        left.y,
        area.width.saturating_sub(left.width + 5),
        30,
    );
    for (title, pane) in [(" Sections ", left), (" Summary ", right)] {
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(title)
            .style(fill)
            .border_style(dim)
            .title_style(Style::new().fg(accent).bg(surface))
            .render(pane, &mut cx);
    }
    let accordion = ac_accordion(tick);
    let inner = Rect::new(left.x + 2, left.y + 2, left.width - 4, left.height - 3);
    accordion.render(inner, &mut cx);
    for section in accordion.layout(inner) {
        let body = AC_SECTIONS[section.index].2;
        for (row, line) in (0..section.body.height).zip(body) {
            cx.put_str(section.body.x, section.body.y + row, line, dim);
        }
    }
    ac_summary(&mut cx, right, tick, theme, [fill, dim]);
    StatusBar::new(" ↑↓ move  ●  enter open or close  ●  q quit")
        .style(Style::new().fg(text).bg(raised))
        .render(
            Rect::new(
                area.x,
                area.y + area.height.saturating_sub(1),
                area.width,
                1,
            ),
            &mut cx,
        );
}

fn ac_summary(cx: &mut RenderCx<'_>, right: Rect, tick: u32, theme: &Theme, styles: [Style; 2]) {
    let [fill, dim] = styles;
    let surface = theme.resolve(Token::Surface0);
    let accent = theme.resolve(Token::Accent);
    let phase = usize::try_from(tick / 60).unwrap_or(3).min(3);
    let (title, badge, _) = AC_SECTIONS[phase];
    cx.put_str(
        right.x + 2,
        right.y + 2,
        title,
        Style::new()
            .fg(accent)
            .bg(surface)
            .with_attrs(Attrs::new().set_bold(true)),
    );
    cx.put_str(right.x + 2, right.y + 3, badge, dim);
    let notes = [
        [
            "Release profile with thin LTO.",
            "Cross build for Apple silicon.",
            "Eight parallel jobs.",
        ],
        [
            "Set for every job in the run.",
            "Colors stay on in the log.",
            "Times are UTC.",
        ],
        [
            "Locked by Cargo.lock.",
            "No yanked releases.",
            "Two minor updates waiting.",
        ],
        [
            "All suites green.",
            "Doctests run on the facade.",
            "Last run 38 s.",
        ],
    ];
    for (row, line) in (0_u16..).zip(notes[phase]) {
        cx.put_str(right.x + 2, right.y + 5 + row, line, fill);
    }
    let runs = [
        ("#1841", "passed", "4m 12s"),
        ("#1840", "passed", "4m 31s"),
        ("#1839", "failed", "2m 03s"),
        ("#1838", "passed", "4m 09s"),
        ("#1837", "passed", "4m 20s"),
        ("#1836", "passed", "4m 44s"),
    ];
    cx.put_str(
        right.x + 2,
        right.y + 11,
        "Recent runs",
        Style::new()
            .fg(accent)
            .bg(surface)
            .with_attrs(Attrs::new().set_bold(true)),
    );
    for (row, (id, result, took)) in (0_u16..).zip(runs) {
        let style = if result == "failed" {
            Style::new().fg(theme.resolve(Token::Fail)).bg(surface)
        } else {
            dim
        };
        cx.put_str(
            right.x + 2,
            right.y + 13 + row,
            &format!("{id}  {result:<7} {took}"),
            style,
        );
    }
}

fn accordion_baseline(_buffer: &mut Buffer, _area: RatRect, _theme: &Theme, _tick: u32) {}

const PG_STAGES: [(&str, u32, u32); 5] = [
    ("Checkout", 0, 40),
    ("Compile", 30, 110),
    ("Test suites", 100, 80),
    ("Package", 170, 30),
    ("Upload", 195, 45),
];

fn pg_progress(tick: u32, start: u32, span: u32) -> f64 {
    if tick <= start {
        0.0
    } else {
        f64::from((tick - start).min(span)) / f64::from(span)
    }
}

fn pg_bar(index: usize, tick: u32) -> ProgressBar<'static> {
    let (_, start, span) = PG_STAGES[index];
    let done = pg_progress(tick, start, span);
    match index {
        1 => ProgressBar::new()
            .segment(0.5_f64.min(done), ProgressTone::Accent)
            .segment((done - 0.5).max(0.0) * 0.9, ProgressTone::Ok)
            .segment(if done >= 1.0 { 0.04 } else { 0.0 }, ProgressTone::Fail),
        2 => ProgressBar::new()
            .segment(done * 0.78, ProgressTone::Ok)
            .segment(done * 0.15, ProgressTone::Warn)
            .segment(
                if done > 0.6 { (done - 0.6) * 0.1 } else { 0.0 },
                ProgressTone::Fail,
            ),
        4 if done > 0.0 && done < 1.0 => ProgressBar::new().indeterminate(tick),
        4 if done >= 1.0 => ProgressBar::new().segment(1.0, ProgressTone::Ok),
        _ => ProgressBar::new().segment(done, ProgressTone::Ok),
    }
}

fn progress_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let raised = theme.resolve(Token::Surface1);
    let text = theme.resolve(Token::TextPrimary);
    let accent = theme.resolve(Token::Accent);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let fill = Style::new().fg(text).bg(surface);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let head = Style::new()
        .fg(accent)
        .bg(raised)
        .with_attrs(Attrs::new().set_bold(true));
    cx.put_str(area.x, area.y, &" ".repeat(usize::from(area.width)), head);
    cx.put_str(
        area.x + 1,
        area.y,
        "Release pipeline  ·  lunatui v0.1.15",
        head,
    );
    let left = Rect::new(area.x + 2, area.y + 2, 74, 24);
    let right = Rect::new(
        left.x + left.width + 1,
        left.y,
        area.width.saturating_sub(left.width + 5),
        24,
    );
    for (title, pane) in [(" Stages ", left), (" Log ", right)] {
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(title)
            .style(fill)
            .border_style(dim)
            .title_style(Style::new().fg(accent).bg(surface))
            .render(pane, &mut cx);
    }
    pg_stages(&mut cx, left, tick, [fill, dim]);
    pg_log(&mut cx, right, tick, [fill, dim]);
    let total: f64 = (0..PG_STAGES.len())
        .map(|i| pg_progress(tick, PG_STAGES[i].1, PG_STAGES[i].2))
        .sum::<f64>()
        / 5.0;
    let overall = Rect::new(area.x + 2, left.y + 25, area.width.saturating_sub(4), 1);
    ProgressBar::new()
        .ratio(total)
        .label("Overall ")
        .show_percent(true)
        .render(overall, &mut cx);
    StatusBar::new(" r rerun stage  ●  l follow log  ●  q quit")
        .style(Style::new().fg(text).bg(raised))
        .render(
            Rect::new(
                area.x,
                area.y + area.height.saturating_sub(1),
                area.width,
                1,
            ),
            &mut cx,
        );
}

fn pg_stages(cx: &mut RenderCx<'_>, area: Rect, tick: u32, styles: [Style; 2]) {
    let [fill, dim] = styles;
    let notes = [
        "14 files",
        "412 of 760 crates cached",
        "1,087 tests",
        "2 artifacts",
        "uploading to the release",
    ];
    for (index, (name, start, span)) in PG_STAGES.iter().enumerate() {
        let y = area.y + 2 + u16::try_from(index).unwrap_or(0) * 4;
        let done = pg_progress(tick, *start, *span);
        let state = if done >= 1.0 && index != 4 {
            "done"
        } else if done > 0.0 {
            "running"
        } else {
            "waiting"
        };
        cx.put_str(area.x + 3, y, name, fill);
        cx.put_str(
            area.x + 3 + 56,
            y,
            state,
            if state == "running" { fill } else { dim },
        );
        pg_bar(index, tick)
            .show_percent(index != 4)
            .render(Rect::new(area.x + 3, y + 1, area.width - 6, 1), cx);
        cx.put_str(area.x + 3, y + 2, notes[index], dim);
    }
}

fn pg_log(cx: &mut RenderCx<'_>, area: Rect, tick: u32, styles: [Style; 2]) {
    let [fill, dim] = styles;
    let lines = [
        "checkout main @ 7577d26",
        "fetch 14 files",
        "cargo build --release",
        "compiling lunatui-widgets",
        "compiling lunatui-runtime",
        "cargo test --workspace",
        "test form ... ok",
        "test slider ... ok",
        "test toast ... ok",
        "1 warning: unused import",
        "package lunatui-0.1.15.crate",
        "upload release assets",
    ];
    let shown = usize::try_from(tick / 20).unwrap_or(0).min(lines.len());
    for (row, line) in (0_u16..).zip(&lines[..shown]) {
        let style = if row + 1 == u16::try_from(shown).unwrap_or(0) {
            fill
        } else {
            dim
        };
        cx.put_str(
            area.x + 2,
            area.y + 1 + row,
            clip_display_prefix(
                &format!("{:02}  {line}", row + 1),
                area.width.saturating_sub(4),
            ),
            style,
        );
    }
}

fn progress_baseline(_buffer: &mut Buffer, _area: RatRect, _theme: &Theme, _tick: u32) {}

const SB_ITEMS: [(&str, &str); 22] = [
    ("Accordion", "collapsible sections"),
    ("Breadcrumbs", "path segments"),
    ("Calendar", "month grid"),
    ("Canvas", "braille and block drawing"),
    ("Chart", "lines, scatter, bars"),
    ("Form", "labelled fields"),
    ("Gauge", "filled ratio"),
    ("Key hint bar", "shortcut footer"),
    ("List", "scrolling rows"),
    ("Markdown view", "rendered text"),
    ("Number input", "spin box"),
    ("Progress bar", "eighth blocks"),
    ("Radio group", "one of many"),
    ("Scroll view", "clipped content"),
    ("Slider", "draggable value"),
    ("Sparkline", "tiny trend"),
    ("Split view", "draggable divider"),
    ("Table", "rows and columns"),
    ("Tabs", "pill tabs"),
    ("Toast stack", "timed messages"),
    ("Toggle", "on and off switch"),
    ("Tooltip", "hint by an anchor"),
];

const SB_TYPING: [(u32, &str); 16] = [
    (0, ""),
    (20, "s"),
    (26, "sl"),
    (32, "sli"),
    (38, "slid"),
    (70, "sli"),
    (74, "sl"),
    (78, "s"),
    (82, ""),
    (100, "o"),
    (106, "on"),
    (112, "on "),
    (118, "on s"),
    (124, "on sw"),
    (170, ""),
    (190, "ta"),
];

fn sb_query(tick: u32) -> &'static str {
    SB_TYPING
        .iter()
        .rev()
        .find(|(at, _)| tick >= *at)
        .map_or("", |(_, query)| *query)
}

fn sb_active(tick: u32, total: usize) -> Option<usize> {
    let steps = [200, 212, 224].iter().filter(|at| tick >= **at).count();
    (steps > 0 && total > 0).then(|| (steps - 1) % total)
}

fn sb_texts() -> Vec<String> {
    SB_ITEMS
        .iter()
        .map(|(name, note)| format!("{name:<16}{note}"))
        .collect()
}

fn search_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let raised = theme.resolve(Token::Surface1);
    let text = theme.resolve(Token::TextPrimary);
    let accent = theme.resolve(Token::Accent);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let fill = Style::new().fg(text).bg(surface);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let head = Style::new()
        .fg(accent)
        .bg(raised)
        .with_attrs(Attrs::new().set_bold(true));
    cx.put_str(area.x, area.y, &" ".repeat(usize::from(area.width)), head);
    cx.put_str(area.x + 1, area.y, "Widget catalog  ·  lunatui", head);
    let left = Rect::new(area.x + 2, area.y + 2, 62, 30);
    let right = Rect::new(
        left.x + left.width + 1,
        left.y,
        area.width.saturating_sub(left.width + 5),
        30,
    );
    for (title, pane) in [(" Widgets ", left), (" Details ", right)] {
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(title)
            .style(fill)
            .border_style(dim)
            .title_style(Style::new().fg(accent).bg(surface))
            .render(pane, &mut cx);
    }
    let query = sb_query(tick);
    let texts = sb_texts();
    let found = SearchBox::filter(&texts, query);
    let active = sb_active(tick, found.len());
    SearchBox::new(query, query.len())
        .placeholder("Filter widgets")
        .matches(active, found.len())
        .state(WidgetState::Focused)
        .render(
            Rect::new(left.x + 2, left.y + 1, left.width - 4, 1),
            &mut cx,
        );
    sb_list(&mut cx, left, &texts, &found, active, [fill, dim]);
    sb_details(&mut cx, right, found.get(active.unwrap_or(0)), [fill, dim]);
    StatusBar::new(" type to filter  ●  n next  N previous  ●  esc clear  ●  q quit")
        .style(Style::new().fg(text).bg(raised))
        .render(
            Rect::new(
                area.x,
                area.y + area.height.saturating_sub(1),
                area.width,
                1,
            ),
            &mut cx,
        );
}

fn sb_list(
    cx: &mut RenderCx<'_>,
    pane: Rect,
    texts: &[String],
    found: &[SearchMatch],
    active: Option<usize>,
    styles: [Style; 2],
) {
    let [fill, _] = styles;
    let accent = cx.token(Token::Accent);
    let raised = cx.token(Token::Surface1);
    if found.is_empty() {
        cx.put_str(
            pane.x + 3,
            pane.y + 4,
            "No widget matches that filter.",
            styles[1],
        );
        return;
    }
    for (row, hit) in (0_u16..).zip(found) {
        let y = pane.y + 4 + row;
        let current = active == Some(usize::from(row));
        let base = if current { fill.bg(raised) } else { fill };
        if current {
            cx.put_str(
                pane.x + 1,
                y,
                &" ".repeat(usize::from(pane.width) - 2),
                base,
            );
            cx.put_str(pane.x + 2, y, "›", base.fg(accent));
        }
        let marked = base.fg(accent).with_attrs(Attrs::new().set_bold(true));
        SearchBox::put_highlighted(
            cx,
            (pane.x + 4, y),
            &texts[hit.index],
            &hit.ranges,
            (base, marked),
            pane.width - 8,
        );
    }
}

fn sb_details(cx: &mut RenderCx<'_>, pane: Rect, shown: Option<&SearchMatch>, styles: [Style; 2]) {
    let [fill, dim] = styles;
    let accent = cx.token(Token::Accent);
    let Some(hit) = shown else {
        cx.put_str(pane.x + 2, pane.y + 2, "Nothing selected", dim);
        return;
    };
    let (name, note) = SB_ITEMS[hit.index];
    let slug = name.to_lowercase().replace(' ', "-");
    cx.put_str(
        pane.x + 2,
        pane.y + 2,
        name,
        fill.fg(accent).with_attrs(Attrs::new().set_bold(true)),
    );
    cx.put_str(pane.x + 2, pane.y + 3, note, dim);
    cx.put_str(pane.x + 2, pane.y + 5, "Feature", dim);
    cx.put_str(pane.x + 14, pane.y + 5, &slug, fill);
    cx.put_str(pane.x + 2, pane.y + 7, "Example", dim);
    cx.put_str(
        pane.x + 14,
        pane.y + 7,
        &format!("widget_preview {slug}"),
        fill,
    );
    cx.put_str(pane.x + 2, pane.y + 9, "Matched", dim);
    let spans: Vec<String> = hit
        .ranges
        .iter()
        .map(|r| format!("{}..{}", r.start, r.end))
        .collect();
    cx.put_str(pane.x + 14, pane.y + 9, &spans.join("  "), fill);
}

fn search_baseline(_buffer: &mut Buffer, _area: RatRect, _theme: &Theme, _tick: u32) {}

const HX_LEN: usize = 4096;
const HX_SECTIONS: [(&str, u32, u32); 4] = [
    ("boot", 0x0400, 0x0400),
    ("kernel", 0x0800, 0x0600),
    ("strings", 0x0100, 0x0200),
    ("rootfs", 0x0e00, 0x0200),
];
const HX_REGIONS: [(&str, usize, usize); 8] = [
    ("Header", 0x000, 0x040),
    ("Section table", 0x040, 0x080),
    ("Reserved", 0x080, 0x100),
    ("Strings", 0x100, 0x300),
    ("Erased flash", 0x300, 0x400),
    ("boot", 0x400, 0x800),
    ("kernel", 0x800, 0xe00),
    ("rootfs", 0xe00, 0x1000),
];
const HX_FIELDS: [(&str, usize, usize); 8] = [
    ("magic", 0x00, 0x08),
    ("version", 0x08, 0x0c),
    ("header size", 0x0c, 0x10),
    ("entry point", 0x10, 0x14),
    ("crc32", 0x14, 0x18),
    ("board", 0x18, 0x28),
    ("built", 0x28, 0x38),
    ("reserved", 0x38, 0x40),
];
const HX_LOG: [&str; 9] = [
    "boot: clocks at 480 MHz\n",
    "boot: loading kernel\n",
    "kernel: 2 cores online\n",
    "fs: mounted rootfs read-only\n",
    "net: link up 1000 Mb/s\n",
    "usb: device attached\n",
    "panic: watchdog timeout\n",
    "cfg: using defaults\n",
    "update: image verified\n",
];

fn hx_image() -> Vec<u8> {
    let mut image = vec![0xff_u8; HX_LEN];
    let mut put = |at: usize, bytes: &[u8]| image[at..at + bytes.len()].copy_from_slice(bytes);
    put(0x00, b"LUNAFW\0\x01");
    put(0x08, &0x0002_0e00_u32.to_le_bytes());
    put(0x0c, &0x40_u32.to_le_bytes());
    put(0x10, &0x0800_04c1_u32.to_le_bytes());
    put(0x14, &0x9a3c_51e7_u32.to_le_bytes());
    put(0x18, b"devkit-rev-c\0\0\0\0");
    put(0x28, b"2026-10-28\0\0\0\0\0\0");
    put(0x38, &[0; 8]);
    for (index, (name, offset, size)) in HX_SECTIONS.iter().enumerate() {
        let at = 0x40 + index * 16;
        let mut label = [0_u8; 8];
        label[..name.len()].copy_from_slice(name.as_bytes());
        put(at, &label);
        put(at + 8, &offset.to_le_bytes());
        put(at + 12, &size.to_le_bytes());
    }
    put(0x80, &[0; 0x80]);
    let mut at = 0x100;
    for line in HX_LOG.iter().cycle().take(18) {
        if at + line.len() + 1 > 0x300 {
            break;
        }
        put(at, line.as_bytes());
        put(at + line.len(), &[0]);
        at += line.len() + 1;
    }
    put(at, &vec![0; 0x300 - at]);
    put(0x400, &0x2002_0000_u32.to_le_bytes());
    for vector in 1..16_u32 {
        let at = 0x400 + usize::try_from(vector).unwrap_or(0) * 4;
        put(at, &(0x0800_04c1 + vector * 8).to_le_bytes());
    }
    let mut seed = 0x2545_f491_u32;
    for at in 0x440..HX_LEN {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        put(at, &seed.to_le_bytes()[..1]);
    }
    image
}

fn hx_step(state: &mut HexViewerState, frame: u32) {
    let every = |start: u32, end: u32, step: u32| {
        (start..=end).contains(&frame) && (frame - start).is_multiple_of(step)
    };
    if every(12, 33, 3) || frame == 215 {
        state.move_right(false);
    } else if every(44, 52, 4) || every(116, 140, 4) || every(228, 236, 4) {
        state.move_right(true);
    } else if every(76, 106, 6) {
        state.move_down(false);
    } else if frame == 112 || frame == 154 {
        state.row_start(false);
    } else if frame == 150 || frame == 175 {
        state.page_down(false);
    } else if frame == 160 {
        state.row_end(true);
    } else if every(185, 200, 1) {
        state.scroll_up(1);
    }
}

fn hex_viewer_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let raised = theme.resolve(Token::Surface1);
    let text = theme.resolve(Token::TextPrimary);
    let accent = theme.resolve(Token::Accent);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let fill = Style::new().fg(text).bg(surface);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let head = Style::new()
        .fg(accent)
        .bg(raised)
        .with_attrs(Attrs::new().set_bold(true));
    cx.put_str(area.x, area.y, &" ".repeat(usize::from(area.width)), head);
    cx.put_str(
        area.x + 1,
        area.y,
        "Firmware inspector  ·  devkit-rev-c.bin  ·  4096 bytes",
        head,
    );
    let left = Rect::new(area.x + 2, area.y + 2, 81, 32);
    let right = Rect::new(
        left.x + left.width + 1,
        left.y,
        area.width.saturating_sub(left.width + 5),
        32,
    );
    for (title, pane) in [(" Image ", left), (" Inspector ", right)] {
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(title)
            .style(fill)
            .border_style(dim)
            .title_style(Style::new().fg(accent).bg(surface))
            .render(pane, &mut cx);
    }
    let image = hx_image();
    let view = Rect::new(left.x + 2, left.y + 1, 77, left.height - 2);
    let viewer = HexViewer::new(&image)
        .header(true)
        .state(WidgetState::Focused);
    let mut state = HexViewerState::new();
    viewer.sync(view, &mut state);
    for frame in 1..=tick {
        hx_step(&mut state, frame);
        viewer.sync(view, &mut state);
    }
    viewer.clone().view(&state).render(view, &mut cx);
    hx_inspector(&mut cx, right, &image, &state, theme, [fill, dim]);
    StatusBar::new(" ←→↑↓ move  ●  shift select  ●  pgup pgdn page  ●  wheel scroll  ●  q quit")
        .style(Style::new().fg(text).bg(raised))
        .render(
            Rect::new(
                area.x,
                area.y + area.height.saturating_sub(1),
                area.width,
                1,
            ),
            &mut cx,
        );
}

fn hx_inspector(
    cx: &mut RenderCx<'_>,
    right: Rect,
    image: &[u8],
    state: &HexViewerState,
    theme: &Theme,
    styles: [Style; 2],
) {
    let [fill, dim] = styles;
    let surface = theme.resolve(Token::Surface0);
    let heading = Style::new()
        .fg(theme.resolve(Token::Accent))
        .bg(surface)
        .with_attrs(Attrs::new().set_bold(true));
    let at = state.cursor();
    let read = |len: usize| {
        let mut word = [0_u8; 4];
        let end = (at + len).min(image.len());
        word[..end - at].copy_from_slice(&image[at..end]);
        u32::from_le_bytes(word)
    };
    let byte = image[at];
    let region = HX_REGIONS
        .iter()
        .find(|(_, start, end)| (*start..*end).contains(&at))
        .map_or("", |(name, _, _)| name);
    let field = HX_FIELDS
        .iter()
        .find(|(_, start, end)| (*start..*end).contains(&at))
        .map_or("-", |(name, _, _)| name);
    let shown = if (0x20..=0x7e).contains(&byte) {
        format!("'{}'", char::from(byte))
    } else {
        "-".to_owned()
    };
    let selection = state.selection().map_or_else(
        || "none".to_owned(),
        |range| {
            format!(
                "{} bytes  {:#05x}..{:#05x}",
                range.len(),
                range.start,
                range.end
            )
        },
    );
    let x = right.x + 2;
    cx.put_str(x, right.y + 2, "Cursor", heading);
    let rows = [
        ("offset", format!("{at:#010x}")),
        ("region", region.to_owned()),
        ("field", field.to_owned()),
        ("u8", format!("{byte:#04x}  {byte}")),
        ("u16 le", format!("{:#06x}  {}", read(2), read(2))),
        ("u32 le", format!("{:#010x}", read(4))),
        ("ascii", shown),
        ("bits", format!("{:04b} {:04b}", byte >> 4, byte & 0xf)),
    ];
    for (row, (label, value)) in (0_u16..).zip(rows) {
        cx.put_str(x, right.y + 4 + row, label, dim);
        cx.put_str(x + 9, right.y + 4 + row, &value, fill);
    }
    cx.put_str(x, right.y + 13, "Selection", heading);
    cx.put_str(x, right.y + 15, &selection, fill);
    cx.put_str(x, right.y + 18, "Sections", heading);
    cx.put_str(x, right.y + 20, "name      offset  size", dim);
    for (row, (name, offset, size)) in (0_u16..).zip(HX_SECTIONS) {
        let start = usize::try_from(offset).unwrap_or(0);
        let end = start + usize::try_from(size).unwrap_or(0);
        let style = if (start..end).contains(&at) {
            heading
        } else {
            fill
        };
        cx.put_str(
            x,
            right.y + 21 + row,
            &format!("{name:<9} {offset:#06x}  {size:#06x}"),
            style,
        );
    }
}

fn hex_viewer_baseline(_buffer: &mut Buffer, _area: RatRect, _theme: &Theme, _tick: u32) {}

const LV_LINES: [(LogLevel, &str, &str); 20] = [
    (LogLevel::Info, "api", "GET /v1/orders 200 in 18ms"),
    (LogLevel::Debug, "db", "pool: 14 of 20 connections idle"),
    (
        LogLevel::Info,
        "web",
        "session 4f2a opened for the dashboard",
    ),
    (
        LogLevel::Trace,
        "edge",
        "keepalive probe to 10.2.0.7 answered",
    ),
    (LogLevel::Warn, "db", "slow query on orders took 812ms"),
    (LogLevel::Info, "jobs", "invoice export queued, 3 ahead"),
    (LogLevel::Debug, "api", "cache hit for catalog:eu (ttl 42s)"),
    (
        LogLevel::Error,
        "edge",
        "upstream web-2 refused the connection",
    ),
    (LogLevel::Info, "api", "POST /v1/payments 201 in 64ms"),
    (LogLevel::Warn, "jobs", "retry 2 of 5 for mail digest"),
    (LogLevel::Debug, "db", "vacuum finished, 1.2 GB reclaimed"),
    (
        LogLevel::Info,
        "web",
        "asset bundle 8c1e served from memory",
    ),
    (LogLevel::Trace, "db", "checkpoint wrote 212 pages"),
    (
        LogLevel::Info,
        "edge",
        "web-2 healthy again, back in rotation",
    ),
    (LogLevel::Warn, "api", "rate limit at 90% for key ak_31"),
    (LogLevel::Info, "jobs", "invoice export finished in 4.1s"),
    (LogLevel::Debug, "web", "websocket ping from 3 clients"),
    (
        LogLevel::Error,
        "db",
        "deadlock detected, transaction rolled back",
    ),
    (LogLevel::Info, "api", "GET /v1/status 200 in 2ms"),
    (LogLevel::Trace, "api", "request id 7e19 assigned"),
];

fn lv_buffer(tick: u32) -> LogBuffer {
    let mut buffer = LogBuffer::new(400);
    let count = 36 + tick / 6;
    for index in 0..count {
        let (level, target, message) = LV_LINES[usize::try_from(index * 7 % 20).unwrap_or(0)];
        let second = 31 + index;
        let stamp = format!("12:{:02}:{:02}", 4 + second / 60, second % 60);
        buffer.push(LogEntry::new(level, target, format!("{message} #{index}")).stamp(stamp));
    }
    buffer
}

fn lv_behind(tick: u32) -> usize {
    if !(120..180).contains(&tick) {
        return 0;
    }
    usize::try_from(14_u32.saturating_sub((tick - 120) / 4)).unwrap_or(0)
}

fn lv_query(tick: u32) -> &'static str {
    if tick >= 180 { "orders" } else { "" }
}

fn lv_level(tick: u32) -> LogLevel {
    if (60..120).contains(&tick) {
        LogLevel::Warn
    } else {
        LogLevel::Debug
    }
}

fn lv_view(buffer: &LogBuffer, tick: u32) -> LogView<'_> {
    let behind = lv_behind(tick);
    LogView::new(buffer)
        .target_width(5)
        .min_level(lv_level(tick))
        .query(lv_query(tick))
        .follow(behind == 0)
        .scroll(behind)
}

fn lv_levels(cx: &mut RenderCx<'_>, (x, y): (u16, u16), counts: [usize; 5], shown: LogLevel) {
    let surface = cx.token(Token::Surface0);
    let text = Style::new().fg(cx.token(Token::TextPrimary)).bg(surface);
    let on = Style::new()
        .fg(surface)
        .bg(cx.token(Token::Accent))
        .with_attrs(Attrs::new().set_bold(true));
    let names = ["trace", "debug", "info", "warn", "error"];
    let levels = [
        LogLevel::Trace,
        LogLevel::Debug,
        LogLevel::Info,
        LogLevel::Warn,
        LogLevel::Error,
    ];
    let mut at = x;
    for ((name, count), level) in names.iter().zip(counts).zip(levels) {
        let label = format!(" {name} {count} ");
        let style = if level >= shown {
            on
        } else {
            text.with_attrs(Attrs::new().set_dim(true))
        };
        cx.put_str(at, y, &label, style);
        at += u16::try_from(label.chars().count()).unwrap_or(0) + 1;
    }
}

fn logs_native(grid: &mut Grid, clusters: &mut ClusterTable, theme: &Theme, area: Rect, tick: u32) {
    let surface = theme.resolve(Token::Surface0);
    let raised = theme.resolve(Token::Surface1);
    let text = theme.resolve(Token::TextPrimary);
    let accent = theme.resolve(Token::Accent);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let fill = Style::new().fg(text).bg(surface);
    let head = Style::new()
        .fg(accent)
        .bg(raised)
        .with_attrs(Attrs::new().set_bold(true));
    cx.put_str(area.x, area.y, &" ".repeat(usize::from(area.width)), head);
    cx.put_str(area.x + 1, area.y, "Service logs  ·  lunatui", head);
    let pane = Rect::new(area.x + 2, area.y + 2, area.width - 4, area.height - 5);
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Logs ")
        .style(fill)
        .border_style(fill.with_attrs(Attrs::new().set_dim(true)))
        .title_style(Style::new().fg(accent).bg(surface))
        .render(pane, &mut cx);
    let buffer = lv_buffer(tick);
    let view = lv_view(&buffer, tick);
    lv_levels(
        &mut cx,
        (pane.x + 2, pane.y + 1),
        buffer.counts(),
        lv_level(tick),
    );
    let query = lv_query(tick);
    SearchBox::new(query, query.len())
        .placeholder("Filter messages")
        .matches(None, view.matching())
        .render(
            Rect::new(pane.x + 2, pane.y + 3, pane.width - 4, 1),
            &mut cx,
        );
    let rows = pane.height - 6;
    view.render(
        Rect::new(pane.x + 2, pane.y + 5, pane.width - 4, rows),
        &mut cx,
    );
    let mode = if lv_behind(tick) > 0 {
        "paused, scrolled back"
    } else {
        "following the newest"
    };
    StatusBar::new(format!(
        " {mode}  ●  {} lines  ●  l level  /  filter  ●  q quit",
        buffer.len()
    ))
    .style(Style::new().fg(text).bg(raised))
    .render(
        Rect::new(
            area.x,
            area.y + area.height.saturating_sub(1),
            area.width,
            1,
        ),
        &mut cx,
    );
}

fn logs_baseline(_buffer: &mut Buffer, _area: RatRect, _theme: &Theme, _tick: u32) {}

static PROFILE: OnceLock<Profile> = OnceLock::new();

const PC_REGIONS: [(&str, f64); 5] = [
    ("us-east", 3420.0),
    ("eu-west", 2610.0),
    ("ap-south", 1480.0),
    ("us-west", 1190.0),
    ("sa-east", 540.0),
];

fn pc_ease(tick: u32, from: u32, span: u32) -> f64 {
    let t = f64::from(tick.saturating_sub(from).min(span)) / f64::from(span);
    1.0 - (1.0 - t).powi(3)
}

fn pc_rates(tick: u32) -> [f64; 5] {
    pc_rates_at(f64::from(tick))
}

fn pc_rates_at(time: f64) -> [f64; 5] {
    let phase = time / 18.0;
    let mut rates = [0.0; 5];
    let mut offset = 0.0;
    for (rate, (_, base)) in rates.iter_mut().zip(PC_REGIONS) {
        *rate = base * (1.0 + 0.22 * (phase + offset).sin() + 0.08 * (phase * 2.3 + offset).cos());
        offset += 1.3;
    }
    rates
}

fn pc_latency(rate: f64, total: f64) -> f64 {
    38.0 + 140.0 * rate / total
}

fn pie_native(grid: &mut Grid, clusters: &mut ClusterTable, theme: &Theme, area: Rect, tick: u32) {
    let surface = theme.resolve(Token::Surface0);
    let raised = theme.resolve(Token::Surface1);
    let text = theme.resolve(Token::TextPrimary);
    let accent = theme.resolve(Token::Accent);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let palette = theme.prepare(PROFILE.get().copied().unwrap_or(Profile::TrueColor));
    let mut cx = RenderCx::new(grid, clusters, theme).palette(&palette);
    let fill = Style::new().fg(text).bg(surface);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let head = Style::new()
        .fg(accent)
        .bg(raised)
        .with_attrs(Attrs::new().set_bold(true));
    cx.put_str(area.x, area.y, &" ".repeat(usize::from(area.width)), head);
    cx.put_str(
        area.x + 1,
        area.y,
        "Cluster overview  ·  prod-eu-2  ·  live",
        head,
    );
    let left = Rect::new(area.x + 2, area.y + 2, 70, 32);
    let right = Rect::new(
        left.x + left.width + 1,
        left.y,
        area.width.saturating_sub(left.width + 5),
        32,
    );
    for (title, pane) in [(" Requests by region ", left), (" Regions ", right)] {
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(title)
            .style(fill)
            .border_style(dim)
            .title_style(Style::new().fg(accent).bg(surface))
            .render(pane, &mut cx);
    }
    let picked = (tick >= 60).then(|| usize::try_from((tick - 60) / 36).unwrap_or(0) % 5);
    let rates = pc_rates(tick);
    PieChart::new(
        PC_REGIONS
            .iter()
            .zip(rates)
            .map(|((label, _), rate)| PieSlice::new(*label, rate)),
    )
    .marker(Marker::Braille)
    .cell_aspect(2.3)
    .reveal(pc_ease(tick, 4, 44))
    .highlight(picked)
    .state(WidgetState::Focused)
    .render(
        Rect::new(left.x + 3, left.y + 2, left.width - 6, 28),
        &mut cx,
    );
    pc_table(&mut cx, right, &rates, picked, [fill, dim]);
    pc_trend(&mut cx, right, tick, [fill, dim]);
    StatusBar::new(" ←→ region  ●  r refresh  ●  q quit")
        .style(Style::new().fg(text).bg(raised))
        .render(
            Rect::new(
                area.x,
                area.y + area.height.saturating_sub(1),
                area.width,
                1,
            ),
            &mut cx,
        );
}

fn pc_table(
    cx: &mut RenderCx<'_>,
    pane: Rect,
    rates: &[f64; 5],
    picked: Option<usize>,
    styles: [Style; 2],
) {
    let [fill, dim] = styles;
    let x = pane.x + 3;
    let top = pane.y + 2;
    cx.put_str(x, top, "region       share   req/s    p95", dim);
    let total: f64 = rates.iter().sum();
    let lit = fill
        .fg(cx.token(Token::FocusRing))
        .with_attrs(Attrs::new().set_bold(true));
    for (row, ((name, _), rate)) in (0_u16..).zip(PC_REGIONS.iter().zip(rates)) {
        let share = 100.0 * rate / total;
        let latency = pc_latency(*rate, total);
        let style = if picked == Some(usize::from(row)) {
            lit
        } else {
            fill
        };
        cx.put_str(
            x,
            top + 2 + row,
            &format!("{name:<10} {share:>5.1}%  {rate:>6.0}  {latency:>4.0} ms"),
            style,
        );
    }
    cx.put_str(
        x,
        top + 8,
        &format!("total              {total:>6.0} req/s"),
        fill.with_attrs(Attrs::new().set_bold(true)),
    );
    let (name, rate) = picked.map_or(("All regions", total), |index| {
        (PC_REGIONS[index].0, rates[index])
    });
    cx.put_str(
        x,
        top + 12,
        name,
        fill.fg(cx.token(Token::Accent))
            .with_attrs(Attrs::new().set_bold(true)),
    );
    let detail = if picked.is_some() {
        format!(
            "{:.0}% of traffic  ·  p95 {:.0} ms",
            100.0 * rate / total,
            pc_latency(rate, total)
        )
    } else {
        format!("{} regions  ·  {total:.0} req/s", PC_REGIONS.len())
    };
    cx.put_str(x, top + 13, &detail, dim);
}

fn pc_trend(cx: &mut RenderCx<'_>, pane: Rect, tick: u32, styles: [Style; 2]) {
    const SECONDS: usize = 40;
    let [fill, dim] = styles;
    let mut points = [(0.0, 0.0); SECONDS];
    for (at, point) in (0_u32..).zip(points.iter_mut()) {
        let back = f64::from(u32::try_from(SECONDS).unwrap_or(0) - 1 - at);
        let total: f64 = pc_rates_at(f64::from(tick) - back * 6.0).iter().sum();
        *point = (f64::from(at), total);
    }
    let x = pane.x + 3;
    let top = pane.y + 17;
    cx.put_str(x, top, "total req/s", dim);
    let sets = [Dataset::new(&points)
        .name("total")
        .graph_type(GraphType::Line)
        .marker(Marker::Braille)];
    Chart::new(&sets)
        .background(fill)
        .legend_position(None)
        .x_axis(
            Axis::new()
                .bounds([0.0, 39.0])
                .labels(&["-40s", "-20s", "now"]),
        )
        .y_axis(
            Axis::new()
                .bounds([8000.0, 11000.0])
                .labels(&["8k", "9.5k", "11k"]),
        )
        .render(Rect::new(x, top + 1, pane.width.saturating_sub(6), 12), cx);
}

fn pie_baseline(_buffer: &mut Buffer, _area: RatRect, _theme: &Theme, _tick: u32) {}

const DV_DIFF: &str = "--- a/src/grid.rs
+++ b/src/grid.rs
@@ -12,14 +12,17 @@ impl Grid {
     pub fn new(width: u16, height: u16) -> Self {
         let len = usize::from(width) * usize::from(height);
-        let cells = vec![Cell::EMPTY; len];
+        let mut cells = Vec::with_capacity(len);
+        cells.resize(len, Cell::EMPTY);
         Self {
             width,
             height,
             cells,
+            dirty: Rect::new(0, 0, width, height),
         }
     }

-    pub fn get(&self, x: u16, y: u16) -> Option<&Cell> {
-        self.cells.get(self.index(x, y))
+    pub fn get(&self, x: u16, y: u16) -> Option<Cell> {
+        let index = self.index(x, y)?;
+        self.cells.get(index).copied()
     }
@@ -41,10 +44,12 @@ impl Grid {
     pub fn set(&mut self, x: u16, y: u16, cell: Cell) {
-        if x < self.width && y < self.height {
-            let at = usize::from(y) * usize::from(self.width) + usize::from(x);
-            self.cells[at] = cell;
+        let Some(index) = self.index(x, y) else {
+            return;
+        };
+        if self.cells[index] != cell {
+            self.cells[index] = cell;
+            self.mark(x, y);
         }
     }

     pub fn fill(&mut self, area: Rect, style: Style) {
-        for y in area.y..area.y + area.height {
-            for x in area.x..area.x + area.width {
+        for y in area.rows() {
+            for x in area.columns() {
                 self.set(x, y, Cell::styled(' ', style));
             }
         }
     }
";

fn dv_diff() -> Diff {
    Diff::parse(DV_DIFF)
}

fn dv_scroll(tick: u32) -> usize {
    let local = if tick < 110 { tick } else { tick - 110 };
    usize::try_from(local.saturating_sub(15) / 5).unwrap_or(0)
}

fn diff_native(grid: &mut Grid, clusters: &mut ClusterTable, theme: &Theme, area: Rect, tick: u32) {
    let surface = theme.resolve(Token::Surface0);
    let raised = theme.resolve(Token::Surface1);
    let text = theme.resolve(Token::TextPrimary);
    let accent = theme.resolve(Token::Accent);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let fill = Style::new().fg(text).bg(surface);
    let head = Style::new()
        .fg(accent)
        .bg(raised)
        .with_attrs(Attrs::new().set_bold(true));
    cx.put_str(area.x, area.y, &" ".repeat(usize::from(area.width)), head);
    cx.put_str(
        area.x + 1,
        area.y,
        "Review  ·  grid: skip redundant writes",
        head,
    );
    let diff = dv_diff();
    let side = tick >= 110;
    let pane = Rect::new(area.x + 2, area.y + 2, area.width - 4, area.height - 5);
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(format!(" {} ", diff.new_name().trim_start_matches("b/")))
        .style(fill)
        .border_style(fill.with_attrs(Attrs::new().set_dim(true)))
        .title_style(Style::new().fg(accent).bg(surface))
        .render(pane, &mut cx);
    let layout = if side {
        DiffLayout::SideBySide
    } else {
        DiffLayout::Unified
    };
    let inner = Rect::new(pane.x + 2, pane.y + 1, pane.width - 4, pane.height - 2);
    DiffView::new(&diff)
        .layout(layout)
        .scroll(dv_scroll(tick))
        .render(inner, &mut cx);
    let (added, removed) = diff.stats();
    let mode = if side { "side by side" } else { "unified" };
    StatusBar::new(format!(
        " {mode}  ●  +{added} −{removed}  ●  v layout  j/k scroll  ●  q quit"
    ))
    .style(Style::new().fg(text).bg(raised))
    .render(
        Rect::new(
            area.x,
            area.y + area.height.saturating_sub(1),
            area.width,
            1,
        ),
        &mut cx,
    );
}

fn diff_baseline(_buffer: &mut Buffer, _area: RatRect, _theme: &Theme, _tick: u32) {}

fn barchart_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let text = theme.resolve(Token::TextPrimary);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let board = BarBoard::new(&cx, tick).pixels(false);
    let wide = area.width >= 100 && area.height >= 30;
    let body = area.height.saturating_sub(1);
    let top = (body * 55 / 100).max(8);
    let split = if wide {
        area.width * 60 / 100
    } else {
        area.width
    };
    board.requests(Rect::new(area.x, area.y, split, top), &mut cx);
    board.runs(
        Rect::new(area.x, area.y + top, split, body.saturating_sub(top)),
        &mut cx,
    );
    if wide {
        let rest = area.width - split;
        board.latency(Rect::new(area.x + split, area.y, rest, top), &mut cx);
        board.states(
            Rect::new(area.x + split, area.y + top, rest, body.saturating_sub(top)),
            &mut cx,
        );
    }
    Paragraph::new(
        "p50 / p95 / p99 per service  |  latency in ms  |  queue depth: normal, focused, disabled",
    )
    .style(
        Style::new()
            .fg(board.text)
            .bg(board.surface)
            .with_attrs(Attrs::new().set_dim(true)),
    )
    .render(
        Rect::new(area.x + 1, area.y + body, area.width.saturating_sub(2), 1),
        &mut cx,
    );
}

fn barchart_baseline(buffer: &mut Buffer, area: RatRect, _theme: &Theme, tick: u32) {
    if let Some(case) = CASES.iter().find(|case| case.name == "barchart") {
        (case.ratatui)(buffer, area, tick);
    }
}

fn chart_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let text = theme.resolve(Token::TextPrimary);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let board = ChartBoard::new(&cx, tick).pixels(false).braille(false);
    let wide = area.width >= 100 && area.height >= 30;
    let top = (area.height * 55 / 100).max(8);
    let split = if wide {
        area.width * 60 / 100
    } else {
        area.width
    };
    board.throughput(Rect::new(area.x, area.y, split, top), &mut cx);
    board.histogram(
        Rect::new(area.x, area.y + top, split, area.height.saturating_sub(top)),
        &mut cx,
    );
    if wide {
        let rest = area.width - split;
        board.scatter(Rect::new(area.x + split, area.y, rest, top), &mut cx);
        board.saturation(
            Rect::new(
                area.x + split,
                area.y + top,
                rest,
                area.height.saturating_sub(top),
            ),
            &mut cx,
        );
    }
}

fn chart_baseline(buffer: &mut Buffer, area: RatRect, _theme: &Theme, tick: u32) {
    if let Some(case) = CASES.iter().find(|case| case.name == "chart") {
        (case.ratatui)(buffer, area, tick);
    }
}

fn canvas_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    canvas_showcase::render(grid, clusters, theme, area, tick);
}

fn canvas_baseline(buffer: &mut Buffer, area: RatRect, _theme: &Theme, tick: u32) {
    if let Some(case) = CASES.iter().find(|case| case.name == "canvas") {
        (case.ratatui)(buffer, area, tick);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Lunatui,
    Ratatui,
}

struct Options {
    case: String,
    side: Side,
    width: u16,
    height: u16,
    frames: u32,
    fps: u32,
    profile: Profile,
    variant: Variant,
}

fn main() -> io::Result<()> {
    let options = parse(&env::args().skip(1).collect::<Vec<_>>())?;
    PROFILE.get_or_init(|| options.profile);
    let case = CASES.iter().find(|case| case.name == options.case);
    if case.is_none() && native_preview(&options.case).is_none() {
        return Err(invalid(format!("unknown case: {}", options.case)));
    }
    pin_color_output();
    let mut out = io::stdout().lock();
    out.write_all(b"\x1b[?1049h\x1b[?25l")?;
    let result = match options.side {
        Side::Lunatui => play_lunatui(&mut out, case, &options),
        Side::Ratatui => play_ratatui(case, &options),
    };
    out.write_all(b"\x1b[0m\x1b[?25h\x1b[?1049l")?;
    out.flush()?;
    result
}

fn play_lunatui(
    out: &mut impl Write,
    case: Option<&'static Case>,
    options: &Options,
) -> io::Result<()> {
    let theme = Theme::builtin(options.variant);
    let area = Rect::new(0, 0, options.width, options.height);
    let mut previous = Grid::new(options.width, options.height);
    let mut next = Grid::new(options.width, options.height);
    let mut clusters = ClusterTable::new();
    let mut encoder = Encoder::new();
    let period = Duration::from_secs(1) / options.fps;
    let started = Instant::now();
    for tick in 0..options.frames {
        match (native_preview(&options.case), case) {
            (Some(draw), _) => draw(&mut next, &mut clusters, &theme, area, tick),
            (None, Some(case)) => (case.lunatui)(&mut next, &mut clusters, &theme, area, tick),
            (None, None) => return Err(invalid(format!("unknown case: {}", options.case))),
        }
        downsample_grid(&mut next, options.profile);
        let mut frame = b"\x1b[?2026h".to_vec();
        frame.extend_from_slice(encoder.encode_diff(
            &previous,
            &next,
            &clusters,
            ColorMode::TrueColor,
        ));
        frame.extend_from_slice(b"\x1b[?2026l");
        out.write_all(&frame)?;
        out.flush()?;
        std::mem::swap(&mut previous, &mut next);
        pace(started, period, tick);
    }
    Ok(())
}

fn pace(started: Instant, period: Duration, tick: u32) {
    let due = started + period * tick.saturating_add(1);
    thread::sleep(due.saturating_duration_since(Instant::now()));
}

fn play_ratatui(case: Option<&'static Case>, options: &Options) -> io::Result<()> {
    let area = RatRect::new(0, 0, options.width, options.height);
    let theme = Theme::builtin(options.variant);
    let mut terminal = Terminal::with_options(
        CrosstermBackend::new(io::stdout()),
        TerminalOptions {
            viewport: Viewport::Fixed(area),
        },
    )?;
    if preview(&options.case).is_none() && case.is_none() {
        return Err(invalid(format!("unknown case: {}", options.case)));
    }
    let period = Duration::from_secs(1) / options.fps;
    let started = Instant::now();
    for tick in 0..options.frames {
        terminal.draw(|frame| {
            let buffer = frame.buffer_mut();
            match (preview(&options.case), case) {
                (Some(preview), _) => (preview.baseline)(buffer, area, &theme, tick),
                (None, Some(case)) => (case.ratatui)(buffer, area, tick),
                (None, None) => {}
            }
            downsample_buffer(buffer, options.profile);
        })?;
        pace(started, period, tick);
    }
    Ok(())
}

fn rat_color(color: Color) -> RatColor {
    match color {
        Color::Rgb(red, green, blue) => RatColor::Rgb(red, green, blue),
        Color::Indexed(index) => RatColor::Indexed(index),
        _ => RatColor::Reset,
    }
}

fn from_rat(color: RatColor) -> Color {
    match color {
        RatColor::Rgb(red, green, blue) => Color::Rgb(red, green, blue),
        RatColor::Indexed(index) => Color::Indexed(index),
        _ => Color::Default,
    }
}

fn downsample_grid(grid: &mut Grid, profile: Profile) {
    if profile == Profile::TrueColor {
        return;
    }
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            let Some(cell) = grid.get(x, y) else {
                continue;
            };
            let style = cell.style();
            let mapped = Style {
                fg: profile.downsample(style.fg),
                bg: profile.downsample(style.bg),
                attrs: style.attrs,
            };
            if mapped != style {
                grid.set_style(Rect::new(x, y, 1, 1), mapped);
            }
        }
    }
}

fn downsample_buffer(buffer: &mut Buffer, profile: Profile) {
    if profile == Profile::TrueColor {
        return;
    }
    for cell in &mut buffer.content {
        cell.fg = rat_color(profile.downsample(from_rat(cell.fg)));
        cell.bg = rat_color(profile.downsample(from_rat(cell.bg)));
    }
}

fn invalid(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn parse(args: &[String]) -> io::Result<Options> {
    let mut options = Options {
        case: "block".to_owned(),
        side: Side::Lunatui,
        width: 80,
        height: 24,
        frames: 90,
        fps: 20,
        profile: Profile::TrueColor,
        variant: Variant::Dark,
    };
    let mut index = 0;
    while index < args.len() {
        let value = args.get(index + 1);
        let flag = args[index].clone();
        let need = |value: Option<&String>| -> io::Result<String> {
            value
                .cloned()
                .ok_or_else(|| invalid(format!("{flag} needs a value")))
        };
        match args[index].as_str() {
            "--case" => options.case = need(value)?,
            "--side" => {
                options.side = match need(value)?.as_str() {
                    "lunatui" => Side::Lunatui,
                    "ratatui" => Side::Ratatui,
                    other => return Err(invalid(format!("unknown side: {other}"))),
                };
            }
            "--profile" => {
                options.profile = match need(value)?.as_str() {
                    "truecolor" => Profile::TrueColor,
                    "ansi256" => Profile::Ansi256,
                    "ansi16" => Profile::Ansi16,
                    other => return Err(invalid(format!("unknown profile: {other}"))),
                };
            }
            "--theme" => {
                options.variant = match need(value)?.as_str() {
                    "dark" => Variant::Dark,
                    "light" => Variant::Light,
                    other => return Err(invalid(format!("unknown theme: {other}"))),
                };
            }
            "--width" => options.width = positive(&need(value)?, "--width")?,
            "--height" => options.height = positive(&need(value)?, "--height")?,
            "--frames" => options.frames = positive(&need(value)?, "--frames")?,
            "--fps" => options.fps = positive(&need(value)?, "--fps")?,
            flag => return Err(invalid(format!("unknown flag: {flag}"))),
        }
        index += 2;
    }
    Ok(options)
}

fn positive<T: std::str::FromStr + Default + PartialEq>(value: &str, flag: &str) -> io::Result<T> {
    let parsed: T = value
        .parse()
        .map_err(|_| invalid(format!("invalid {flag}")))?;
    if parsed == T::default() {
        return Err(invalid(format!("{flag} must be greater than zero")));
    }
    Ok(parsed)
}

const CB_REGIONS: [&str; 13] = [
    "ap-northeast-1",
    "ap-south-1",
    "ap-southeast-2",
    "ca-central-1",
    "eu-central-1",
    "eu-north-1",
    "eu-west-1",
    "eu-west-2",
    "eu-west-3",
    "sa-east-1",
    "us-east-1",
    "us-east-2",
    "us-west-2",
];

const CB_TYPING: [(u32, &str); 7] = [
    (60, "e"),
    (68, "eu"),
    (76, "eu-"),
    (84, "eu-w"),
    (92, "eu-we"),
    (140, "eu-west-2"),
    (210, ""),
];

fn cb_text(tick: u32) -> &'static str {
    CB_TYPING
        .iter()
        .rev()
        .find(|(at, _)| tick >= *at)
        .map_or("", |(_, text)| *text)
}

fn cb_open(tick: u32) -> bool {
    (20..140).contains(&tick)
}

fn cb_active(tick: u32) -> Option<usize> {
    match tick {
        30..=35 | 60..=99 => Some(0),
        36..=41 | 100..=111 => Some(1),
        42..=59 | 112..=139 => Some(2),
        _ => None,
    }
}

fn combo_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let raised = theme.resolve(Token::Surface1);
    let text = theme.resolve(Token::TextPrimary);
    let accent = theme.resolve(Token::Accent);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let fill = Style::new().fg(text).bg(surface);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let head = Style::new()
        .fg(accent)
        .bg(raised)
        .with_attrs(Attrs::new().set_bold(true));
    cx.put_str(area.x, area.y, &" ".repeat(usize::from(area.width)), head);
    cx.put_str(area.x + 1, area.y, "Deploy  ·  lunatui", head);
    let panel = Rect::new(area.x + 2, area.y + 2, 64, 24);
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" New deployment ")
        .style(fill)
        .border_style(dim)
        .title_style(Style::new().fg(accent).bg(surface))
        .render(panel, &mut cx);
    let value_x = panel.x + 16;
    let rows = [
        (2, "Service", "api-gateway"),
        (4, "Region", ""),
        (6, "Replicas", "3"),
        (8, "Strategy", "rolling, 25% at a time"),
        (10, "Notify", "#releases"),
    ];
    for (dy, label, value) in rows {
        cx.put_str(panel.x + 3, panel.y + dy, label, dim);
        cx.put_str(value_x, panel.y + dy, value, fill);
    }
    let typed = cb_text(tick);
    let focused = (20..150).contains(&tick);
    ComboBox::new(typed, typed.len(), &CB_REGIONS)
        .placeholder("Choose a region")
        .open(cb_open(tick))
        .active(cb_active(tick))
        .max_rows(6)
        .state(if focused {
            WidgetState::Focused
        } else {
            WidgetState::Normal
        })
        .render(Rect::new(value_x, panel.y + 4, 32, 8), &mut cx);
    StatusBar::new(" type to filter  ●  up/down choose  ●  enter select  ●  esc close")
        .style(Style::new().fg(text).bg(raised))
        .render(
            Rect::new(
                area.x,
                area.y + area.height.saturating_sub(1),
                area.width,
                1,
            ),
            &mut cx,
        );
}

fn combo_baseline(_buffer: &mut Buffer, _area: RatRect, _theme: &Theme, _tick: u32) {}

fn dp_day(year: i32, month: u8, day: u8) -> Date {
    Date::from_ymd(year, month, day).unwrap_or(Date::UNIX_EPOCH)
}

fn dp_cursor(tick: u32) -> Date {
    let start = dp_day(2026, 10, 14);
    let days = match tick {
        0..=29 => 0,
        30..=89 => (1 + (tick - 30) / 6).min(10),
        _ => 10 + 7 * (1 + (tick - 90) / 20).min(3),
    };
    DatePicker::shift_days(start, i32::try_from(days).unwrap_or(0))
}

fn datepick_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let raised = theme.resolve(Token::Surface1);
    let text = theme.resolve(Token::TextPrimary);
    let accent = theme.resolve(Token::Accent);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let fill = Style::new().fg(text).bg(surface);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let head = Style::new()
        .fg(accent)
        .bg(raised)
        .with_attrs(Attrs::new().set_bold(true));
    cx.put_str(area.x, area.y, &" ".repeat(usize::from(area.width)), head);
    cx.put_str(area.x + 1, area.y, "Trips  ·  lunatui", head);
    let panel = Rect::new(area.x + 2, area.y + 2, 54, 14);
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Choose your dates ")
        .style(fill)
        .border_style(dim)
        .title_style(Style::new().fg(accent).bg(surface))
        .render(panel, &mut cx);
    let anchor = dp_day(2026, 10, 14);
    let cursor = dp_cursor(tick);
    let confirmed = tick >= 150;
    let focus_right = cursor.month() == 11;
    for (index, month) in [10_u8, 11].into_iter().enumerate() {
        let shown = if cursor.month() == month {
            cursor
        } else {
            dp_day(2026, month, 1)
        };
        let mut picker = DatePicker::new(shown)
            .today(anchor)
            .min(dp_day(2026, 10, 10))
            .max(dp_day(2026, 11, 28))
            .state(if !confirmed && (index == 1) == focus_right {
                WidgetState::Focused
            } else {
                WidgetState::Normal
            });
        if tick >= 20 {
            picker = picker.range(anchor, cursor);
        }
        let x = panel.x + 3 + u16::try_from(index).unwrap_or(0) * 25;
        picker.render(
            Rect::new(x, panel.y + 2, DatePicker::WIDTH, DatePicker::HEIGHT),
            &mut cx,
        );
    }
    let summary = if tick < 20 {
        "Move with the arrow keys, press enter to start".to_owned()
    } else {
        let nights = (0..=62)
            .find(|days| DatePicker::shift_days(anchor, *days) >= cursor)
            .unwrap_or(0);
        format!(
            "{} {} to {} {}  ·  {nights} nights{}",
            &anchor.month_name()[..3],
            anchor.day(),
            &cursor.month_name()[..3],
            cursor.day(),
            if confirmed { "  ·  confirmed" } else { "" }
        )
    };
    cx.put_str(panel.x + 3, panel.y + panel.height - 3, &summary, fill);
    StatusBar::new(" arrows day  ●  pgup/pgdn month  ●  enter select  ●  esc cancel")
        .style(Style::new().fg(text).bg(raised))
        .render(
            Rect::new(
                area.x,
                area.y + area.height.saturating_sub(1),
                area.width,
                1,
            ),
            &mut cx,
        );
}

fn datepick_baseline(_buffer: &mut Buffer, _area: RatRect, _theme: &Theme, _tick: u32) {}

const NOTIFY_FEED: [(u32, ToastKind, &str, &str); 7] = [
    (0, ToastKind::Info, "Signed in on a new device", "11:30"),
    (0, ToastKind::Success, "Cache warmed on edge-3", "11:41"),
    (0, ToastKind::Success, "Release notes saved", "11:58"),
    (40, ToastKind::Success, "Build 4821 finished", "12:04"),
    (
        70,
        ToastKind::Warning,
        "Disk almost full on build-2",
        "12:09",
    ),
    (100, ToastKind::Error, "Deploy to staging failed", "12:15"),
    (120, ToastKind::Info, "Retry queued for staging", "12:16"),
];

fn notify_clock(tick: u32) -> Duration {
    Duration::from_millis(u64::from(tick) * 33)
}

fn notify_log(tick: u32) -> NotificationLog {
    let mut log = NotificationLog::new(16);
    for (arrival, kind, text, stamp) in NOTIFY_FEED {
        if tick >= arrival {
            log.push(kind, text, stamp);
        }
    }
    if tick >= 140 {
        let visited = usize::try_from(((tick - 140) / 14).min(3)).unwrap_or(0);
        for position in 0..=visited {
            let source = NOTIFY_FEED.len() - 1 - position;
            log.mark_read(NotificationId(u32::try_from(source).unwrap_or(0)));
        }
    }
    if tick >= 200 {
        log.dismiss(NotificationId(3));
    }
    if tick >= 215 {
        log.mark_all_read();
    }
    log
}

fn notify_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let raised = theme.resolve(Token::Surface1);
    let text = theme.resolve(Token::TextPrimary);
    let accent = theme.resolve(Token::Accent);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let fill = Style::new().fg(text).bg(surface);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let body_height = area.height.saturating_sub(1);
    let deploys = Rect::new(area.x + 1, area.y, 58, body_height);
    let panel = Rect::new(area.x + 61, area.y, 58, body_height);
    for (title, pane) in [(" Deploys ", deploys), (" Inbox ", panel)] {
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(title)
            .style(fill)
            .border_style(dim)
            .title_style(Style::new().fg(accent).bg(surface))
            .render(pane, &mut cx);
    }
    let services = ["api", "web", "jobs", "edge", "docs", "staging", "billing"];
    for (row, name) in (0_u16..).zip(services) {
        let state = match (name, tick) {
            ("staging", 100..=119) => "failed",
            ("staging", 120..) => "retrying",
            _ => "passing",
        };
        let line = format!(" {name:<12}{state:<10}#{}", 4800 + u32::from(row) * 3);
        cx.put_str(deploys.x + 2, deploys.y + 2 + row * 2, &line, fill);
    }
    let log = notify_log(tick);
    let selected = (tick >= 140).then(|| {
        let step = usize::try_from(((tick - 140) / 14).min(3)).unwrap_or(0);
        step.min(log.len().saturating_sub(1))
    });
    NotificationCenter::new(&log)
        .selected(selected)
        .state(WidgetState::Focused)
        .render(
            Rect::new(panel.x + 1, panel.y + 1, panel.width - 2, panel.height - 2),
            &mut cx,
        );
    let mut toasts = ToastStack::new();
    let now = notify_clock(tick);
    for (arrival, kind, text, _) in NOTIFY_FEED {
        let at = notify_clock(arrival);
        if arrival > 0 && now >= at {
            toasts.push(
                at,
                Some(Duration::from_millis(900)),
                ToastScope::Pane(1),
                kind,
                text,
            );
        }
    }
    toasts.render(now, area, |id| (id == 1).then_some(deploys), &mut cx);
    StatusBar::new(" ↑↓ select  ●  x dismiss  ●  m mark all read  ●  esc close")
        .style(Style::new().fg(text).bg(raised))
        .render(
            Rect::new(area.x, area.y + body_height, area.width, 1),
            &mut cx,
        );
}

fn notify_baseline(_buffer: &mut Buffer, _area: RatRect, _theme: &Theme, _tick: u32) {}

fn tags_state(tick: u32) -> (Vec<String>, String, Option<usize>) {
    let mut tags = vec!["rust".to_owned(), "tui".to_owned()];
    let mut text = String::new();
    let typing = |word: &str, start: u32, per: u32, tick: u32| -> usize {
        let typed = usize::try_from(tick.saturating_sub(start) / per + 1).unwrap_or(0);
        typed.min(word.len())
    };
    let words = [
        ("async", 20_u32, 85_u32),
        ("widgets", 100, 165),
        ("grid", 175, 205),
    ];
    for (word, start, done) in words {
        if tick >= done {
            tags.push(word.to_owned());
        } else if tick >= start {
            word[..typing(word, start, 12, tick)].clone_into(&mut text);
        }
    }
    let selected = (tick >= 215).then(|| tags.len() - 1);
    if tick >= 232 {
        tags.pop();
        return (tags, text, None);
    }
    (tags, text, selected)
}

fn tags_native(grid: &mut Grid, clusters: &mut ClusterTable, theme: &Theme, area: Rect, tick: u32) {
    let surface = theme.resolve(Token::Surface0);
    let raised = theme.resolve(Token::Surface1);
    let text = theme.resolve(Token::TextPrimary);
    let accent = theme.resolve(Token::Accent);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let fill = Style::new().fg(text).bg(surface);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let head = Style::new()
        .fg(accent)
        .bg(raised)
        .with_attrs(Attrs::new().set_bold(true));
    cx.put_str(area.x, area.y, &" ".repeat(usize::from(area.width)), head);
    cx.put_str(area.x + 1, area.y, "Journal  ·  new entry", head);
    let panel = Rect::new(area.x + 2, area.y + 2, 56, 16);
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Tag your entry ")
        .style(fill)
        .border_style(dim)
        .title_style(Style::new().fg(accent).bg(surface))
        .render(panel, &mut cx);
    cx.put_str(panel.x + 3, panel.y + 2, "Title", dim);
    cx.put_str(
        panel.x + 3,
        panel.y + 3,
        "Zero-allocation rendering notes",
        fill,
    );
    cx.put_str(panel.x + 3, panel.y + 5, "Tags", dim);
    let (tags, typed, selected) = tags_state(tick);
    let box_area = Rect::new(panel.x + 3, panel.y + 6, 36, 4);
    Block::bordered()
        .border_style(dim)
        .style(fill)
        .render(Rect::new(box_area.x - 1, box_area.y - 1, 38, 6), &mut cx);
    TagInput::new(&tags, typed.as_str(), typed.len())
        .placeholder("Type a tag and press enter")
        .selected(selected)
        .state(WidgetState::Focused)
        .render(box_area, &mut cx);
    let hint = match selected {
        Some(_) => "Selected tag: press delete to remove it",
        None if typed.is_empty() => "Type a tag, then press enter to add it",
        None => "Press enter to add the tag",
    };
    cx.put_str(panel.x + 3, panel.y + 12, hint, dim);
    StatusBar::new(" enter add  ●  ← select tag  ●  delete remove  ●  esc cancel")
        .style(Style::new().fg(text).bg(raised))
        .render(
            Rect::new(
                area.x,
                area.y + area.height.saturating_sub(1),
                area.width,
                1,
            ),
            &mut cx,
        );
}

fn tags_baseline(_buffer: &mut Buffer, _area: RatRect, _theme: &Theme, _tick: u32) {}
