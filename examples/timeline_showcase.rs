// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, ClusterTable, Grid, Rect, Style};
use lunatui_theme::{Theme, Token};
use lunatui_widgets::{
    BigText, Block, BorderType, Gauge, LogBuffer, LogEntry, LogLevel, LogView, Paragraph,
    PixelSize, RenderCx, Timeline, TimelineItem, TimelineItemId, TimelineLane, TimelineStatus,
    TimelineTick, TimelineView, Widget, WidgetState,
};

/// Times are minutes from Monday, November 2, 00:00.
const HOUR: i64 = 60;
const DAY: i64 = 24 * HOUR;
/// The close view spans eight days; the overview spans sixteen.
const CLOSE: i64 = 8 * DAY;
const OVERVIEW: i64 = 16 * DAY;

const LANES: [&str; 12] = [
    "core/cells",
    "core/width",
    "widgets/timeline",
    "widgets/heatmap",
    "widgets/dialog",
    "runtime/input",
    "runtime/async",
    "web/sandbox",
    "web/site",
    "release/docs",
    "release/qa",
    "release/ship",
];

const DAYS: [&str; 17] = [
    "Mon 02", "Tue 03", "Wed 04", "Thu 05", "Fri 06", "Sat 07", "Sun 08", "Mon 09", "Tue 10",
    "Wed 11", "Thu 12", "Fri 13", "Sat 14", "Sun 15", "Mon 16", "Tue 17", "Wed 18",
];
const WEEKS: [&str; 3] = ["Nov 02", "Nov 09", "Nov 16"];

/// The current time: Thursday 02:00, advancing 18 minutes a frame.
fn now(tick: u32) -> i64 {
    3 * DAY + 2 * HOUR + i64::from(tick) * 18
}

const fn overview(tick: u32) -> bool {
    tick >= 150 && tick < 225
}

fn view(tick: u32) -> TimelineView {
    if overview(tick) {
        TimelineView::new(0, OVERVIEW)
    } else {
        let start = (now(tick) - 2 * DAY - 16 * HOUR).max(0);
        TimelineView::new(start, start + CLOSE)
    }
}

fn timeline_progress(tick: u32) -> u8 {
    u8::try_from((52 + tick / 5).min(100)).unwrap_or(100)
}

fn h(day: i64, hour: i64) -> i64 {
    day * DAY + hour * HOUR
}

/// One lane's items: (start, end, label, status, progress, milestone).
type Plan = (i64, i64, &'static str, TimelineStatus, Option<u8>, bool);

fn plan(lane: usize, tick: u32) -> Vec<Plan> {
    use TimelineStatus::{AtRisk, Blocked, Done, Planned};
    let timeline_status = if timeline_progress(tick) >= 100 {
        Done
    } else {
        Planned
    };
    let dialog_status = if tick < 120 { Blocked } else { AtRisk };
    let dialog_progress = if tick < 120 {
        30
    } else {
        30 + (tick - 120) / 6
    };
    let dialog_progress = u8::try_from(dialog_progress.min(80)).unwrap_or(80);
    match lane {
        0 => vec![
            (h(0, 0), h(2, 12), "grid diff", Done, None, false),
            (h(2, 20), h(5, 10), "cluster table", Done, Some(100), false),
        ],
        1 => vec![
            (h(1, 0), h(4, 4), "width tables", Done, None, false),
            (h(4, 8), 0, "unicode 17", Done, None, true),
        ],
        2 => vec![(
            h(2, 0),
            h(7, 12),
            "timeline widget",
            timeline_status,
            Some(timeline_progress(tick)),
            false,
        )],
        3 => vec![
            (h(1, 6), h(2, 22), "labels", Done, None, false),
            (h(3, 8), h(5, 20), "pixel mode", AtRisk, Some(55), false),
        ],
        4 => vec![(
            h(2, 12),
            h(6, 6),
            "focus trap",
            dialog_status,
            Some(dialog_progress),
            false,
        )],
        5 => vec![(h(4, 0), h(8, 8), "mouse drag", Planned, Some(20), false)],
        6 => vec![(h(5, 20), h(10, 20), "task pool", Planned, None, false)],
        7 => vec![
            (h(3, 0), h(7, 2), "wasm sandbox", AtRisk, Some(60), false),
            (h(7, 8), 0, "demo", Planned, None, true),
        ],
        8 => vec![(h(7, 2), h(11, 16), "site refresh", Planned, None, false)],
        9 => vec![(h(8, 8), h(12, 12), "guides", Planned, None, false)],
        10 => vec![(h(10, 10), h(13, 18), "clean VM pass", Planned, None, false)],
        _ => vec![
            (h(8, 0), 0, "beta", Done, None, true),
            (h(14, 0), 0, "0.2 GA", Planned, None, true),
        ],
    }
}

fn item(&(start, end, label, status, progress, milestone): &Plan) -> TimelineItem<'static> {
    let item = if milestone {
        TimelineItem::milestone(start)
    } else {
        TimelineItem::span(start, end)
    };
    let item = item.label(label).status(status);
    match progress {
        Some(percent) => item.progress(percent),
        None => item,
    }
}

/// The app-owned selection, moved the way arrow keys would move it.
fn selected(tick: u32) -> TimelineItemId {
    let (lane, item) = match tick {
        0..90 => (2, 0),
        90..150 => (4, 0),
        150..225 => (11, 1),
        _ => (7, 0),
    };
    TimelineItemId { lane, item }
}

fn clock(at: i64) -> String {
    let day = usize::try_from(at.div_euclid(DAY))
        .unwrap_or(0)
        .min(DAYS.len() - 1);
    let minutes = at.rem_euclid(DAY);
    format!("{} {:02}:{:02}", DAYS[day], minutes / HOUR, minutes % HOUR)
}

fn label(cx: &mut RenderCx<'_>, area: Rect, text: impl Into<String>, style: Style) {
    Paragraph::new(text).style(style).render(area, cx);
}

fn panel(cx: &mut RenderCx<'_>, area: Rect, title: &str, style: Style) -> Rect {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(title)
        .style(style)
        .border_style(style.fg(cx.token(Token::Accent)));
    block.render(area, cx);
    block.inner(area)
}

pub fn render(grid: &mut Grid, clusters: &mut ClusterTable, theme: &Theme, area: Rect, tick: u32) {
    *grid = Grid::new(grid.width(), grid.height());
    let mut cx = RenderCx::new(grid, clusters, theme);
    let ground = Style::new()
        .fg(cx.token(Token::TextPrimary))
        .bg(cx.token(Token::Surface0));
    let raised = ground.bg(cx.token(Token::Surface1));
    Block::new().style(ground).render(area, &mut cx);
    Block::new()
        .style(raised)
        .render(Rect::new(area.x, area.y, area.width, 1), &mut cx);
    label(
        &mut cx,
        Rect::new(area.x, area.y, area.width, 1),
        format!(
            " release plan / lunatui 0.2{:>width$}",
            format!("NOW  {} ", clock(now(tick))),
            width = usize::from(area.width).saturating_sub(27)
        ),
        raised.with_attrs(Attrs::new().set_bold(true)),
    );
    label(
        &mut cx,
        Rect::new(area.x + 2, area.y + 2, area.width.saturating_sub(4), 1),
        format!(
            "RELEASE TRAIN     12 workstreams / 3 milestones     view: {}",
            if overview(tick) {
                "16 days, weekly ticks"
            } else {
                "8 days, following now"
            }
        ),
        ground,
    );
    if area.width < 100 || area.height < 36 {
        schedule(
            &mut cx,
            Rect::new(
                area.x + 1,
                area.y + 4,
                area.width.saturating_sub(2),
                area.height.saturating_sub(5),
            ),
            tick,
        );
        return;
    }
    // The schedule panel fits 12 lanes, the axis header, and the legend exactly.
    let body_height = 20;
    let schedule_area = Rect::new(area.x + 2, area.y + 4, area.width - 34, body_height);
    let inner = panel(&mut cx, schedule_area, " Schedule ", ground);
    schedule(
        &mut cx,
        Rect::new(
            inner.x + 1,
            inner.y + 1,
            inner.width.saturating_sub(2),
            inner.height.saturating_sub(4),
        ),
        tick,
    );
    legend(
        &mut cx,
        Rect::new(
            inner.x + 1,
            inner.y + inner.height - 2,
            inner.width.saturating_sub(2),
            1,
        ),
    );
    let inspector_area = Rect::new(area.x + area.width - 30, area.y + 4, 28, body_height);
    let inner = panel(&mut cx, inspector_area, " Selected ", ground);
    inspector(&mut cx, inner, tick, ground);
    let event_area = Rect::new(area.x + 2, area.y + 25, area.width - 4, area.height - 27);
    let inner = panel(&mut cx, event_area, " Plan events ", ground);
    activity(&mut cx, inner, tick);
    label(
        &mut cx,
        Rect::new(area.x, area.y + area.height - 1, area.width, 1),
        " h/l pan   +/- zoom   j/k select   t today                      plan synced from the release board",
        raised,
    );
}

fn schedule(cx: &mut RenderCx<'_>, area: Rect, tick: u32) {
    let plans: [Vec<Plan>; 12] = std::array::from_fn(|lane| plan(lane, tick));
    let items: [Vec<TimelineItem<'static>>; 12] =
        std::array::from_fn(|lane| plans[lane].iter().map(item).collect());
    let lanes: [TimelineLane<'_>; 12] =
        std::array::from_fn(|lane| TimelineLane::new(LANES[lane], &items[lane]));
    let days: [TimelineTick<'static>; 17] = std::array::from_fn(|day| {
        TimelineTick::new(i64::try_from(day).unwrap_or(0) * DAY, DAYS[day])
    });
    let weeks: [TimelineTick<'static>; 3] = std::array::from_fn(|week| {
        TimelineTick::new(i64::try_from(week).unwrap_or(0) * 7 * DAY, WEEKS[week])
    });
    Timeline::new(&lanes)
        .ticks(if overview(tick) { &weeks } else { &days })
        .view(view(tick))
        .now(Some(now(tick)))
        .selected(Some(selected(tick)))
        .state(WidgetState::Focused)
        .render(area, cx);
}

fn legend(cx: &mut RenderCx<'_>, area: Rect) {
    let items = [
        TimelineItem::span(0, 9).label(" planned"),
        TimelineItem::span(10, 16)
            .label(" done")
            .status(TimelineStatus::Done),
        TimelineItem::span(17, 26)
            .label(" at risk")
            .status(TimelineStatus::AtRisk),
        TimelineItem::span(27, 36)
            .label(" blocked")
            .status(TimelineStatus::Blocked),
        TimelineItem::milestone(38).label("milestone"),
    ];
    let lanes = [TimelineLane::new("", &items)];
    let width = i64::from(area.width.max(1));
    Timeline::new(&lanes)
        .view(TimelineView::new(0, width))
        .render(area, cx);
}

fn inspector(cx: &mut RenderCx<'_>, area: Rect, tick: u32, ground: Style) {
    let id = selected(tick);
    let plans = plan(id.lane, tick);
    let Some(&(start, end, name, status, progress, milestone)) = plans.get(id.item) else {
        return;
    };
    let (token, word) = match status {
        TimelineStatus::Planned => (Token::Accent, "PLANNED / on track"),
        TimelineStatus::Done => (Token::Ok, "DONE / shipped"),
        TimelineStatus::AtRisk => (Token::Warn, "AT RISK / review owed"),
        TimelineStatus::Blocked => (Token::Fail, "BLOCKED / waiting on fix"),
    };
    let tone = cx.token(token);
    let bold = ground.with_attrs(Attrs::new().set_bold(true));
    label(
        cx,
        Rect::new(area.x + 1, area.y + 1, area.width - 2, 1),
        name,
        bold,
    );
    label(
        cx,
        Rect::new(area.x + 1, area.y + 2, area.width - 2, 1),
        LANES[id.lane],
        ground,
    );
    let percent = if milestone {
        if status == TimelineStatus::Done {
            100
        } else {
            0
        }
    } else {
        progress.unwrap_or(if status == TimelineStatus::Done {
            100
        } else {
            0
        })
    };
    BigText::new(&format!("{percent:>3}%"))
        .pixel_size(PixelSize::Quadrant)
        .style(ground.fg(tone))
        .render(Rect::new(area.x + 2, area.y + 4, area.width - 4, 4), cx);
    label(
        cx,
        Rect::new(area.x + 1, area.y + 9, area.width - 2, 1),
        word,
        ground.fg(tone),
    );
    Gauge::new(f32::from(percent) / 100.0)
        .gauge_style(ground.fg(tone))
        .track_style(ground.bg(cx.token(Token::Surface1)))
        .sub_cell(true)
        .render(Rect::new(area.x + 1, area.y + 11, area.width - 2, 1), cx);
    let dates = if milestone {
        format!("due    {}", clock(start))
    } else {
        format!("start  {}\nend    {}", clock(start), clock(end))
    };
    label(
        cx,
        Rect::new(area.x + 1, area.y + 13, area.width - 2, 2),
        dates,
        ground,
    );
    let remaining = if milestone { start } else { end } - now(tick);
    label(
        cx,
        Rect::new(area.x + 1, area.y + 16, area.width - 2, 1),
        if remaining > 0 {
            format!(
                "{}d {:02}h remaining",
                remaining / DAY,
                remaining % DAY / HOUR
            )
        } else {
            "past due date".to_owned()
        },
        ground,
    );
}

fn activity(cx: &mut RenderCx<'_>, area: Rect, tick: u32) {
    let mut log = LogBuffer::new(7);
    for (at, level, message) in [
        (
            0,
            LogLevel::Info,
            "Plan synced: 12 workstreams, 3 milestones",
        ),
        (
            45,
            LogLevel::Info,
            "widgets/timeline moved past 60%; review booked",
        ),
        (
            90,
            LogLevel::Error,
            "widgets/dialog blocked: focus trap leaks on nested popups",
        ),
        (
            120,
            LogLevel::Warn,
            "widgets/dialog unblocked; fix in review, still at risk",
        ),
        (
            150,
            LogLevel::Info,
            "Zoomed out to 16 days to check the 0.2 GA date",
        ),
        (225, LogLevel::Info, "Back to the 8-day view; following now"),
        (
            240,
            LogLevel::Info,
            "widgets/timeline done; web/sandbox demo is next",
        ),
    ] {
        if tick >= at {
            log.push(LogEntry::new(level, "", message).stamp(clock(now(at))));
        }
    }
    LogView::new(&log)
        .target_width(0)
        .follow(true)
        .render(area, cx);
}
