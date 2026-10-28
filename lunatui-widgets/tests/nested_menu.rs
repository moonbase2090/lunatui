// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT
//! Nested menu behavior and tui-menu cell parity checks.

use lunatui_core::{ClusterTable, Color, Grid, Rect};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Menu, MenuItem, MenuState, RenderCx, Widget, WidgetState};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect as RatRect;
use ratatui::style::{Color as RatColor, Modifier, Style as RatStyle};
use ratatui::widgets::StatefulWidget;
use tui_menu::{Menu as RatMenu, MenuItem as RatMenuItem, MenuState as RatMenuState};

#[derive(Clone, Copy)]
enum Scenario {
    Inactive,
    RootOpen,
    NestedOpen,
}

fn native_state(scenario: Scenario) -> MenuState<&'static str> {
    let mut state = MenuState::new(vec![
        MenuItem::group(
            "File",
            vec![
                MenuItem::item("New", "new"),
                MenuItem::item("Open", "open"),
                MenuItem::group(
                    "Open recent",
                    vec![
                        MenuItem::item("file_1.txt", "recent-1"),
                        MenuItem::item("file_2.txt", "recent-2"),
                    ],
                ),
                MenuItem::item("Save as", "save-as"),
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
    match scenario {
        Scenario::Inactive => {}
        Scenario::RootOpen => {
            state.activate();
        }
        Scenario::NestedOpen => {
            state.activate();
            state.down();
            state.down();
            state.down();
            state.right();
        }
    }
    state
}

fn ratatui_state(scenario: Scenario) -> RatMenuState<&'static str> {
    let mut state = RatMenuState::new(vec![
        RatMenuItem::group(
            "File",
            vec![
                RatMenuItem::item("New", "new"),
                RatMenuItem::item("Open", "open"),
                RatMenuItem::group(
                    "Open recent",
                    vec![
                        RatMenuItem::item("file_1.txt", "recent-1"),
                        RatMenuItem::item("file_2.txt", "recent-2"),
                    ],
                ),
                RatMenuItem::item("Save as", "save-as"),
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
    match scenario {
        Scenario::Inactive => {}
        Scenario::RootOpen => state.activate(),
        Scenario::NestedOpen => {
            state.activate();
            state.down();
            state.down();
            state.down();
            state.right();
        }
    }
    state
}

fn paint_native(
    state: &MenuState<&'static str>,
    theme: &Theme,
    width: u16,
    height: u16,
) -> (Grid, ClusterTable) {
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, theme);
    Menu::new(state)
        .state(WidgetState::Focused)
        .render(Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

fn paint_ratatui(
    state: &mut RatMenuState<&'static str>,
    theme: &Theme,
    width: u16,
    height: u16,
) -> Buffer {
    let area = RatRect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    let normal = RatStyle::new()
        .fg(to_rat_color(theme.resolve(Token::TextPrimary)))
        .bg(to_rat_color(theme.resolve(Token::SurfaceRaised)));
    let selected = RatStyle::new()
        .fg(to_rat_color(theme.resolve(Token::TextMutedOnFocus)))
        .bg(to_rat_color(theme.resolve(Token::FocusFill)));
    StatefulWidget::render(
        RatMenu::new()
            .default_style(normal)
            .highlight(selected)
            .dropdown_width(20),
        area,
        &mut buffer,
        state,
    );
    buffer
}

fn to_rat_color(color: Color) -> RatColor {
    match color {
        Color::Default => RatColor::Reset,
        Color::Rgb(red, green, blue) => RatColor::Rgb(red, green, blue),
        Color::Indexed(index) => RatColor::Indexed(index),
        Color::Token(_) => RatColor::Reset,
    }
}

fn from_rat_color(color: RatColor) -> Color {
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

fn assert_same_cells(grid: &Grid, clusters: &ClusterTable, buffer: &Buffer) {
    for y in 0..buffer.area.height {
        let native = grid.row_display(y, clusters);
        let baseline = (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol().to_owned())
            .collect::<String>();
        assert_eq!(native, baseline, "row {y}");
        for x in 0..buffer.area.width {
            let cell = grid.get(x, y).expect("cell exists");
            let expected = &buffer[(x, y)];
            let actual_style = cell.style();
            assert_eq!(
                actual_style.fg,
                from_rat_color(expected.fg),
                "fg at {x},{y}"
            );
            assert_eq!(
                actual_style.bg,
                from_rat_color(expected.bg),
                "bg at {x},{y}"
            );
            assert_eq!(
                actual_style.attrs.bold(),
                expected.modifier.contains(Modifier::BOLD),
                "bold at {x},{y}"
            );
            assert_eq!(
                actual_style.attrs.dim(),
                expected.modifier.contains(Modifier::DIM),
                "dim at {x},{y}"
            );
        }
    }
}

#[test]
fn matches_tui_menu_cells_for_inactive_dropdown_and_nested_states() {
    let theme = Theme::builtin(Variant::Dark);
    for (width, height) in [(80, 24), (200, 60)] {
        for scenario in [Scenario::Inactive, Scenario::RootOpen, Scenario::NestedOpen] {
            let native_state = native_state(scenario);
            let mut baseline_state = ratatui_state(scenario);
            let (grid, clusters) = paint_native(&native_state, &theme, width, height);
            let buffer = paint_ratatui(&mut baseline_state, &theme, width, height);
            assert_same_cells(&grid, &clusters, &buffer);
        }
    }
}

#[test]
fn activation_and_down_skip_disabled_items() {
    let mut state = MenuState::new(vec![
        MenuItem::item("Unavailable", "unavailable").disabled(true),
        MenuItem::group(
            "File",
            vec![
                MenuItem::item("Unavailable child", "unavailable-child").disabled(true),
                MenuItem::item("Open", "open"),
            ],
        ),
    ]);

    assert!(state.activate());
    assert_eq!(state.selected_path(), &[1]);
    assert!(state.down());
    assert_eq!(state.selected_path(), &[1, 1]);
    assert_eq!(state.selected().map(MenuItem::label), Some("Open"));
}

#[test]
fn selecting_a_leaf_queues_an_action_until_drained() {
    let mut state = MenuState::new(vec![MenuItem::item("Open", "open")]);
    assert!(state.activate());

    state.select();
    assert_eq!(
        state.drain_events().collect::<Vec<_>>(),
        [lunatui_widgets::MenuEvent::Selected("open")]
    );
    assert_eq!(state.drain_events().count(), 0);
}

#[test]
fn reset_closes_nested_levels_without_dropping_queued_actions() {
    let mut state = MenuState::new(vec![MenuItem::group(
        "File",
        vec![MenuItem::group(
            "Open recent",
            vec![MenuItem::item("notes.txt", "open-recent")],
        )],
    )]);
    assert!(state.activate());
    assert!(state.down());
    assert!(state.right());
    state.select();

    assert!(state.reset());
    assert!(state.selected_path().is_empty());
    assert_eq!(
        state.drain_events().collect::<Vec<_>>(),
        [lunatui_widgets::MenuEvent::Selected("open-recent")]
    );
}
