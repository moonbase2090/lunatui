// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Block, Date, Monthly, Padding, RenderCx, WeekStart, Widget, WidgetState};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect as RatRect;
use ratatui::style::{Color as RatColor, Modifier, Style as RatStyle};
use ratatui::widgets::Widget as RatWidget;
use ratatui::widgets::calendar::{DateStyler, Monthly as RatMonthly};
use time::{Date as TimeDate, Month};

struct Empty;

impl DateStyler for Empty {
    fn get_style(&self, _date: TimeDate) -> RatStyle {
        RatStyle::new()
    }
}

struct Mark {
    date: TimeDate,
    style: RatStyle,
}

impl DateStyler for Mark {
    fn get_style(&self, date: TimeDate) -> RatStyle {
        if date == self.date {
            self.style
        } else {
            RatStyle::new()
        }
    }
}

fn ymd(year: i32, month: u8, day: u8) -> Result<Date, String> {
    Date::from_ymd(year, month, day).ok_or_else(|| format!("bad date {year}-{month}-{day}"))
}

fn rat_ymd(year: i32, month: Month, day: u8) -> Result<TimeDate, String> {
    TimeDate::from_calendar_date(year, month, day)
        .map_err(|_| format!("bad date {year}-{month:?}-{day}"))
}

fn paint(
    monthly: &Monthly<impl lunatui_widgets::DateStyle>,
    width: u16,
    height: u16,
) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    monthly.render(Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

fn rat_paint(monthly: &RatMonthly<'_, impl DateStyler>, width: u16, height: u16) -> Buffer {
    let area = RatRect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    RatWidget::render(monthly, area, &mut buffer);
    buffer
}

fn rows(grid: &Grid, clusters: &ClusterTable) -> Vec<String> {
    (0..grid.height())
        .map(|row| grid.row_display(row, clusters))
        .collect()
}

fn rat_rows(buffer: &Buffer) -> Vec<String> {
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect()
        })
        .collect()
}

fn rat_color(color: RatColor) -> Color {
    match color {
        RatColor::Reset => Color::Default,
        RatColor::Rgb(red, green, blue) => Color::Rgb(red, green, blue),
        RatColor::Indexed(index) => Color::Indexed(index),
        RatColor::Black => Color::Indexed(0),
        RatColor::Red => Color::Indexed(1),
        RatColor::Green => Color::Indexed(2),
        RatColor::Yellow => Color::Indexed(3),
        RatColor::Blue => Color::Indexed(4),
        RatColor::Magenta => Color::Indexed(5),
        RatColor::Cyan => Color::Indexed(6),
        RatColor::Gray => Color::Indexed(7),
        RatColor::DarkGray => Color::Indexed(8),
        RatColor::LightRed => Color::Indexed(9),
        RatColor::LightGreen => Color::Indexed(10),
        RatColor::LightYellow => Color::Indexed(11),
        RatColor::LightBlue => Color::Indexed(12),
        RatColor::LightMagenta => Color::Indexed(13),
        RatColor::LightCyan => Color::Indexed(14),
        RatColor::White => Color::Indexed(15),
    }
}

fn assert_same_cells(grid: &Grid, clusters: &ClusterTable, buffer: &Buffer) -> Result<(), String> {
    let ours = rows(grid, clusters);
    let theirs = rat_rows(buffer);
    if ours != theirs {
        return Err(format!("symbols\nours={ours:?}\ntheirs={theirs:?}"));
    }
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let Some(cell) = grid.get(x, y) else {
                return Err(format!("missing cell at {x},{y}"));
            };
            let want = &buffer[(x, y)];
            let style = cell.style();
            if style.fg != rat_color(want.fg) || style.bg != rat_color(want.bg) {
                return Err(format!(
                    "color at {x},{y}: {:?} vs {:?}",
                    (style.fg, style.bg),
                    (rat_color(want.fg), rat_color(want.bg))
                ));
            }
            if style.attrs.bold() != want.modifier.contains(Modifier::BOLD)
                || style.attrs.dim() != want.modifier.contains(Modifier::DIM)
            {
                return Err(format!("attrs at {x},{y}"));
            }
        }
    }
    Ok(())
}

fn styled(red: u8, green: u8, blue: u8) -> (Style, RatStyle) {
    (
        Style::new().fg(Color::Rgb(red, green, blue)).bg(Color::Rgb(
            red.wrapping_add(1),
            green,
            blue,
        )),
        RatStyle::new()
            .fg(RatColor::Rgb(red, green, blue))
            .bg(RatColor::Rgb(red.wrapping_add(1), green, blue)),
    )
}

#[test]
fn week_counts_match_ratatui() -> Result<(), String> {
    let feb_2015 = ymd(2015, 2, 1)?;
    assert_eq!(Monthly::new(feb_2015, ()).height(), 4);
    assert_eq!(Monthly::new(feb_2015, ()).width(), 21);
    let decorated = Monthly::new(feb_2015, ())
        .show_month_header(Style::new())
        .show_weekdays_header(Style::new());
    assert_eq!(decorated.height(), 6);

    let feb_2024 = ymd(2024, 2, 29)?;
    assert_eq!(Monthly::new(feb_2024, ()).height(), 5);
    let apr_2023 = ymd(2023, 4, 9)?;
    assert_eq!(Monthly::new(apr_2023, ()).height(), 6);
    let dec_2024 = ymd(2024, 12, 1)?;
    assert_eq!(Monthly::new(dec_2024, ()).height(), 5);
    Ok(())
}

#[test]
fn block_padding_grows_the_frame() -> Result<(), String> {
    let date = ymd(2015, 2, 1)?;
    let padded = Padding {
        left: 2,
        right: 3,
        top: 1,
        bottom: 2,
    };
    let calendar = Monthly::new(date, ()).block(Block::bordered().padding(padded));
    assert_eq!(calendar.width(), 28);
    assert_eq!(calendar.height(), 9);

    let bordered = Monthly::new(ymd(2024, 2, 1)?, ()).block(Block::bordered());
    assert_eq!(bordered.width(), 23);
    assert_eq!(bordered.height(), 7);
    Ok(())
}

#[test]
fn february_2015_matches_ratatui() -> Result<(), String> {
    let (days, rat_days) = styled(20, 30, 40);
    let (month, rat_month) = styled(50, 60, 70);
    let (week, rat_week) = styled(80, 90, 100);
    let (event, rat_event) = styled(200, 40, 40);
    let date = ymd(2015, 2, 14)?;
    let events = [(ymd(2015, 2, 14)?, event)];
    let ours = Monthly::new(date, events.as_slice())
        .default_style(days)
        .show_month_header(month)
        .show_weekdays_header(week);
    let rat = RatMonthly::new(
        rat_ymd(2015, Month::February, 14)?,
        Mark {
            date: rat_ymd(2015, Month::February, 14)?,
            style: rat_event,
        },
    )
    .default_style(rat_days)
    .show_month_header(rat_month)
    .show_weekdays_header(rat_week);
    let (grid, clusters) = paint(&ours, 30, 8);
    let buffer = rat_paint(&rat, 30, 8);
    assert_same_cells(&grid, &clusters, &buffer)?;
    assert!(rows(&grid, &clusters)[0].contains("February 2015"));
    Ok(())
}

#[test]
fn april_2023_surrounding_days_match_ratatui() -> Result<(), String> {
    let (days, rat_days) = styled(10, 20, 30);
    let (around, rat_around) = styled(90, 90, 90);
    let date = ymd(2023, 4, 1)?;
    let ours = Monthly::new(date, ())
        .default_style(days)
        .show_surrounding(around)
        .show_month_header(days)
        .show_weekdays_header(days);
    let rat = RatMonthly::new(rat_ymd(2023, Month::April, 1)?, Empty)
        .default_style(rat_days)
        .show_surrounding(rat_around)
        .show_month_header(rat_days)
        .show_weekdays_header(rat_days);
    let (grid, clusters) = paint(&ours, 24, 10);
    let buffer = rat_paint(&rat, 24, 10);
    assert_same_cells(&grid, &clusters, &buffer)?;
    Ok(())
}

#[test]
fn december_rolls_into_january_without_a_sixth_week() -> Result<(), String> {
    let (days, rat_days) = styled(1, 2, 3);
    let date = ymd(2024, 12, 31)?;
    let ours = Monthly::new(date, ()).default_style(days);
    let rat = RatMonthly::new(rat_ymd(2024, Month::December, 31)?, Empty).default_style(rat_days);
    let (grid, clusters) = paint(&ours, 21, 6);
    let buffer = rat_paint(&rat, 21, 6);
    assert_same_cells(&grid, &clusters, &buffer)?;
    Ok(())
}

#[test]
fn minimal_area_stays_inside_the_rect() -> Result<(), String> {
    let date = ymd(1984, 1, 1)?;
    let (grid, clusters) = paint(&Monthly::new(date, ()), 1, 1);
    let buffer = rat_paint(
        &RatMonthly::new(rat_ymd(1984, Month::January, 1)?, Empty),
        1,
        1,
    );
    assert_same_cells(&grid, &clusters, &buffer)?;
    assert_eq!(rows(&grid, &clusters), [" "]);
    let (grid, clusters) = paint(&Monthly::new(date, ()), 0, 0);
    assert_eq!(grid.row_display(0, &clusters), "");
    Ok(())
}

#[test]
fn rejects_impossible_days() {
    assert!(Date::from_ymd(2023, 2, 29).is_none());
    assert!(Date::from_ymd(2024, 2, 29).is_some());
    assert!(Date::from_ymd(2024, 13, 1).is_none());
    assert!(Date::from_ymd(2024, 4, 31).is_none());
}

#[test]
fn themed_states_color_the_selected_day() -> Result<(), String> {
    let date = ymd(2023, 4, 9)?;
    let theme = Theme::builtin(Variant::Dark);
    for state in [
        WidgetState::Normal,
        WidgetState::Focused,
        WidgetState::Disabled,
    ] {
        let (grid, _) = paint(&Monthly::new(date, ()).state(state), 21, 8);
        let Some(selected) = grid.get(2, 4).map(lunatui_core::Cell::style) else {
            return Err("missing selected day".to_owned());
        };
        let Some(other) = grid.get(5, 4).map(lunatui_core::Cell::style) else {
            return Err("missing neighboring day".to_owned());
        };
        let Some(outside) = grid.get(2, 2).map(lunatui_core::Cell::style) else {
            return Err("missing surrounding day".to_owned());
        };
        match state {
            WidgetState::Normal => {
                assert_eq!(selected.fg, theme.resolve(Token::Accent));
                assert_eq!(other.fg, theme.resolve(Token::TextPrimary));
                assert!(outside.attrs.dim());
            }
            WidgetState::Focused => {
                assert_eq!(selected.fg, theme.resolve(Token::FocusRing));
                assert!(selected.attrs.bold());
                assert_eq!(other.fg, theme.resolve(Token::TextPrimary));
            }
            WidgetState::Disabled => {
                assert_eq!(selected.fg, theme.resolve(Token::TextPrimary));
                assert!(selected.attrs.dim());
                assert_eq!(selected.attrs, Attrs::new().set_dim(true));
            }
            _ => return Err("untested state".to_owned()),
        }
    }
    Ok(())
}

#[test]
fn last_event_style_wins() -> Result<(), String> {
    let date = ymd(2024, 6, 2)?;
    let first = Style::new().fg(Color::Rgb(1, 0, 0));
    let second = Style::new().fg(Color::Rgb(0, 1, 0));
    let events = [(date, first), (date, second)];
    let (grid, _) = paint(
        &Monthly::new(date, events.as_slice())
            .default_style(Style::new().fg(Color::Rgb(9, 9, 9)).bg(Color::Rgb(8, 8, 8))),
        21,
        6,
    );
    let Some(cell) = grid.get(2, 1).map(lunatui_core::Cell::style) else {
        return Err("missing event cell".to_owned());
    };
    assert_eq!(cell.fg, Color::Rgb(0, 1, 0));
    Ok(())
}

#[test]
fn monday_start_renders_event_dot_in_date_gutter() -> Result<(), String> {
    let date = ymd(2024, 6, 3)?;
    let event_style = Style::new().fg(Color::Rgb(185, 167, 240));
    let events = [(date, event_style)];
    let monthly = Monthly::new(date, events.as_slice())
        .week_start(WeekStart::Monday)
        .event_markers(true)
        .show_weekdays_header(Style::new());
    let (grid, clusters) = paint(&monthly, 21, 6);
    let rows = rows(&grid, &clusters);
    assert_eq!(rows[0], " Mo Tu We Th Fr Sa Su");
    assert!(
        rows[2].starts_with("• 3"),
        "unexpected second week: {}",
        rows[2]
    );
    let Some(dot_style) = grid.get(0, 2).map(lunatui_core::Cell::style) else {
        return Err("missing first-day event marker".to_owned());
    };
    assert_eq!(dot_style.fg, Color::Rgb(185, 167, 240));
    Ok(())
}

#[test]
fn monday_start_counts_four_week_february() -> Result<(), String> {
    let date = ymd(2021, 2, 1)?;
    let monday = Monthly::new(date, ()).week_start(WeekStart::Monday);
    let sunday = Monthly::new(date, ()).week_start(WeekStart::Sunday);
    assert_eq!(monday.height(), 4);
    assert_eq!(sunday.height(), 5);

    let decorated = monday
        .show_month_header(Style::new())
        .show_weekdays_header(Style::new());
    assert_eq!(decorated.height(), 6);
    Ok(())
}

#[test]
fn monday_markers_skip_events_on_surrounding_days() -> Result<(), String> {
    let visible = ymd(2024, 5, 31)?;
    let current = ymd(2024, 6, 3)?;
    let events = [(visible, Style::new().fg(Color::Rgb(185, 167, 240)))];
    let calendar = Monthly::new(current, events.as_slice())
        .week_start(WeekStart::Monday)
        .event_markers(true);
    let (grid, clusters) = paint(&calendar, 21, 6);
    let rows = rows(&grid, &clusters);
    assert_eq!(rows[0].chars().nth(12), Some(' '));
    Ok(())
}

#[test]
fn weekend_style_colors_weekends_under_event_styles() -> Result<(), String> {
    let weekend = Style::new().fg(Color::Rgb(143, 124, 224));
    let event = Style::new().fg(Color::Rgb(185, 167, 240));
    let events = [(ymd(2024, 6, 8)?, event)];
    let monthly = Monthly::new(ymd(2024, 6, 3)?, events.as_slice())
        .week_start(WeekStart::Monday)
        .default_style(Style::new().fg(Color::Rgb(236, 231, 218)))
        .weekend_style(weekend)
        .show_weekdays_header(Style::new());
    let (grid, clusters) = paint(&monthly, 21, 7);
    let rows = rows(&grid, &clusters);
    assert_eq!(rows[1], "                 1  2");
    assert_eq!(rows[2], "  3  4  5  6  7  8  9");
    let fg = |x, y| grid.get(x, y).map(|cell| cell.style().fg);
    assert_eq!(fg(17, 1), Some(weekend.fg), "Saturday 1");
    assert_eq!(fg(20, 1), Some(weekend.fg), "Sunday 2");
    assert_eq!(fg(17, 2), Some(event.fg), "Saturday 8 has an event");
    assert_eq!(fg(20, 2), Some(weekend.fg), "Sunday 9");
    assert_eq!(fg(14, 2), Some(Color::Rgb(236, 231, 218)), "Friday 7");
    Ok(())
}

#[test]
fn selected_style_covers_the_day_and_one_cell_on_each_side() -> Result<(), String> {
    let chip = Style::new()
        .fg(Color::Rgb(10, 13, 28))
        .bg(Color::Rgb(185, 167, 240));
    let event = Style::new().fg(Color::Rgb(243, 226, 176));
    let events = [(ymd(2024, 6, 4)?, event)];
    let monthly = Monthly::new(ymd(2024, 6, 4)?, events.as_slice())
        .week_start(WeekStart::Monday)
        .selected_style(chip)
        .show_weekdays_header(Style::new());
    let (grid, _) = paint(&monthly, 21, 7);
    let style = |x| {
        grid.get(x, 2)
            .map(lunatui_core::Cell::style)
            .unwrap_or_default()
    };
    for x in 3..=6 {
        assert_eq!(style(x).bg, chip.bg, "x={x}");
    }
    assert_eq!(
        style(4).fg,
        chip.fg,
        "the selected style wins over the event style"
    );
    assert_ne!(style(2).bg, chip.bg);
    assert_ne!(style(7).bg, chip.bg);
    Ok(())
}

#[test]
fn selected_chip_stays_inside_the_area_in_the_last_column() -> Result<(), String> {
    let chip = Style::new().bg(Color::Rgb(185, 167, 240));
    let monthly = Monthly::new(ymd(2024, 6, 9)?, ())
        .week_start(WeekStart::Monday)
        .selected_style(chip)
        .show_weekdays_header(Style::new());
    let (grid, _) = paint(&monthly, 22, 7);
    let bg = |x| grid.get(x, 2).map(|cell| cell.style().bg);
    assert_eq!(bg(18), Some(chip.bg));
    assert_eq!(bg(20), Some(chip.bg));
    assert_ne!(bg(21), Some(chip.bg));
    Ok(())
}
