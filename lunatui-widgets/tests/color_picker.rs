// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    ColorPicker, ColorPickerFocus, ColorPickerState, RenderCx, Widget, WidgetState,
};

fn draw(picker: &ColorPicker<'_>, area: Rect, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    picker.render(area, &mut cx);
    (grid, clusters)
}

fn rows(picker: &ColorPicker<'_>, area: Rect, width: u16, height: u16) -> Vec<String> {
    let (grid, clusters) = draw(picker, area, width, height);
    (0..height)
        .map(|y| grid.row_display(y, &clusters))
        .collect()
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y).map(Cell::style).unwrap_or_default()
}

fn token(token: Token) -> Color {
    Theme::builtin(Variant::Dark).resolve(token)
}

const AREA: Rect = Rect::new(0, 0, 20, 6);

#[test]
fn rgb_round_trips_through_the_state() {
    for rgb in [
        (0, 0, 0),
        (255, 255, 255),
        (255, 0, 0),
        (0, 255, 0),
        (0, 0, 255),
        (18, 52, 86),
        (250, 128, 114),
        (1, 2, 3),
        (127, 127, 128),
    ] {
        let state = ColorPickerState::from_rgb(rgb.0, rgb.1, rgb.2);
        assert_eq!(state.rgb(), rgb, "{rgb:?}");
        assert_eq!(state.color(), Color::Rgb(rgb.0, rgb.1, rgb.2));
    }
}

#[test]
fn hsv_of_primaries_and_secondaries() {
    let hue = |r, g, b| ColorPickerState::from_rgb(r, g, b).hue().round();
    assert_eq!(hue(255, 0, 0), 0.0);
    assert_eq!(hue(255, 255, 0), 60.0);
    assert_eq!(hue(0, 255, 0), 120.0);
    assert_eq!(hue(0, 255, 255), 180.0);
    assert_eq!(hue(0, 0, 255), 240.0);
    assert_eq!(hue(255, 0, 255), 300.0);
    let gray = ColorPickerState::from_rgb(128, 128, 128);
    assert_eq!(gray.saturation(), 0.0);
    assert!((gray.value() - 128.0 / 255.0).abs() < 1e-6);
}

#[test]
fn a_gray_keeps_the_hue_already_chosen() {
    let mut state = ColorPickerState::from_rgb(0, 0, 255);
    state.set_rgb(90, 90, 90);
    assert_eq!(state.hue().round(), 240.0);
    state.set_rgb(0, 0, 0);
    assert_eq!(state.hue().round(), 240.0);
}

#[test]
fn hex_is_uppercase_with_a_hash() {
    assert_eq!(&ColorPickerState::from_rgb(18, 171, 205).hex(), b"#12ABCD");
    assert_eq!(&ColorPickerState::from_rgb(0, 0, 0).hex(), b"#000000");
}

#[test]
fn nudging_the_square_moves_saturation_and_value() {
    let mut state = ColorPickerState::from_rgb(255, 0, 0);
    state.set_rgb(255, 128, 128);
    let saturation = state.saturation();
    state.nudge(-10, 0);
    assert!((state.saturation() - (saturation - 0.10)).abs() < 1e-5);
    let value = state.value();
    state.nudge(0, -5);
    assert!((state.value() - (value + 0.05).min(1.0)).abs() < 1e-5);
    state.nudge(0, 500);
    assert_eq!(state.value(), 0.0);
    state.nudge(500, 0);
    assert_eq!(state.saturation(), 1.0);
    state.nudge(-500, 0);
    assert_eq!(state.saturation(), 0.0);
}

#[test]
fn nudging_the_hue_wraps_in_both_directions() {
    let mut state = ColorPickerState::from_rgb(255, 0, 0);
    state.set_focus(ColorPickerFocus::Hue);
    state.nudge(-1, 0);
    assert_eq!(state.hue().round(), 359.0);
    state.nudge(2, 0);
    assert_eq!(state.hue().round(), 1.0);
    state.nudge(0, 120);
    assert_eq!(state.hue().round(), 121.0);
    assert!(state.hue() < 360.0);
}

#[test]
fn the_hex_entry_ignores_nudges() {
    let mut state = ColorPickerState::from_rgb(10, 20, 30);
    state.set_focus(ColorPickerFocus::Hex);
    state.nudge(40, 40);
    assert_eq!(state.rgb(), (10, 20, 30));
}

#[test]
fn focus_cycles_both_ways_and_drops_a_half_typed_hex() {
    let mut state = ColorPickerState::new();
    assert_eq!(state.focus(), ColorPickerFocus::Square);
    state.next_focus();
    assert_eq!(state.focus(), ColorPickerFocus::Hue);
    state.next_focus();
    assert_eq!(state.focus(), ColorPickerFocus::Hex);
    assert!(state.type_digit('a'));
    assert!(state.is_editing());
    state.next_focus();
    assert_eq!(state.focus(), ColorPickerFocus::Square);
    assert!(!state.is_editing());
    assert_eq!(state.typed().len(), 0);
    state.previous_focus();
    assert_eq!(state.focus(), ColorPickerFocus::Hex);
    state.previous_focus();
    assert_eq!(state.focus(), ColorPickerFocus::Hue);
}

#[test]
fn typing_takes_only_hex_digits_on_the_entry_and_at_most_six() {
    let mut state = ColorPickerState::new();
    assert!(!state.type_digit('f'), "the square is focused");
    state.set_focus(ColorPickerFocus::Hex);
    assert!(!state.type_digit('g'));
    assert!(!state.type_digit('#'));
    for digit in "1a2B3c".chars() {
        assert!(state.type_digit(digit));
    }
    assert!(!state.type_digit('4'));
    assert_eq!(state.typed(), b"1A2B3C");
}

#[test]
fn backspace_removes_the_last_digit_and_ends_editing_when_empty() {
    let mut state = ColorPickerState::new();
    state.set_focus(ColorPickerFocus::Hex);
    assert!(!state.backspace());
    state.type_digit('1');
    state.type_digit('2');
    assert!(state.backspace());
    assert_eq!(state.typed(), b"1");
    assert!(state.is_editing());
    assert!(state.backspace());
    assert!(!state.is_editing());
    assert!(!state.backspace());
}

#[test]
fn commit_applies_six_digits_and_expands_three() {
    let mut state = ColorPickerState::new();
    state.set_focus(ColorPickerFocus::Hex);
    for digit in "12abcd".chars() {
        state.type_digit(digit);
    }
    assert!(state.commit_hex());
    assert_eq!(state.rgb(), (0x12, 0xAB, 0xCD));
    assert!(!state.is_editing());
    for digit in "f80".chars() {
        state.type_digit(digit);
    }
    assert!(state.commit_hex());
    assert_eq!(state.rgb(), (0xFF, 0x88, 0x00));
}

#[test]
fn commit_with_the_wrong_digit_count_keeps_the_digits() {
    let mut state = ColorPickerState::from_rgb(1, 2, 3);
    state.set_focus(ColorPickerFocus::Hex);
    for digit in "abcd".chars() {
        state.type_digit(digit);
    }
    assert!(!state.commit_hex());
    assert_eq!(state.rgb(), (1, 2, 3));
    assert_eq!(state.typed(), b"ABCD");
    assert!(state.is_editing());
}

#[test]
fn the_layout_puts_square_bar_and_hex_row_apart() {
    assert_eq!(ColorPicker::square_area(AREA), Rect::new(0, 0, 16, 5));
    assert_eq!(ColorPicker::hue_area(AREA), Rect::new(17, 0, 2, 5));
    assert_eq!(ColorPicker::hex_area(AREA), Rect::new(0, 5, 20, 1));
    let offset = Rect::new(3, 2, 20, 6);
    assert_eq!(ColorPicker::square_area(offset), Rect::new(3, 2, 16, 5));
    assert_eq!(ColorPicker::hue_area(offset), Rect::new(20, 2, 2, 5));
    assert_eq!(ColorPicker::hex_area(offset), Rect::new(3, 7, 20, 1));
}

#[test]
fn a_narrow_or_short_area_leaves_out_the_square_and_bar() {
    let narrow = Rect::new(0, 0, 5, 6);
    assert_eq!(ColorPicker::square_area(narrow).width, 0);
    assert_eq!(ColorPicker::hue_area(narrow).width, 0);
    let one_row = Rect::new(0, 0, 20, 1);
    assert_eq!(ColorPicker::square_area(one_row).height, 0);
    assert_eq!(ColorPicker::hex_area(one_row), Rect::new(0, 0, 20, 1));
    let state = ColorPickerState::from_rgb(1, 2, 3);
    let picker = ColorPicker::new().view(&state);
    assert_eq!(
        rows(&picker, one_row, 20, 1)[0].trim_end(),
        "     #010203 H210"
    );
}

#[test]
fn the_square_runs_white_to_the_hue_across_and_down_to_black() {
    let state = ColorPickerState::new();
    let picker = ColorPicker::new().view(&state);
    let (grid, _) = draw(&picker, AREA, 20, 6);
    let square = ColorPicker::square_area(AREA);
    let corner = |x: u16, y: u16| style_at(&grid, square.x + x, square.y + y);
    assert_eq!(corner(0, 0).fg, Color::Rgb(255, 255, 255));
    assert_eq!(corner(14, 0).fg, Color::Rgb(255, 17, 17));
    for x in 0..square.width {
        assert_eq!(corner(x, 4).bg, Color::Rgb(0, 0, 0), "column {x}");
    }
}

#[test]
fn the_hue_bar_starts_red_and_ends_just_short_of_red() {
    let state = ColorPickerState::from_rgb(0, 0, 255);
    let picker = ColorPicker::new().view(&state);
    let (grid, _) = draw(&picker, AREA, 20, 6);
    let bar = ColorPicker::hue_area(AREA);
    assert_eq!(style_at(&grid, bar.x, bar.y).fg, Color::Rgb(255, 0, 0));
    let last = style_at(&grid, bar.x, bar.y + bar.height - 1).bg;
    assert_eq!(last, Color::Rgb(255, 0, 4));
    assert_eq!(
        style_at(&grid, bar.x, bar.y).bg,
        style_at(&grid, bar.x + 1, bar.y).bg
    );
}

#[test]
fn the_knob_marks_the_color_and_the_pointer_marks_the_hue() {
    let state = ColorPickerState::from_rgb(0, 0, 255);
    let picker = ColorPicker::new().view(&state);
    let lines = rows(&picker, AREA, 20, 6);
    assert_eq!(lines[0].chars().nth(15), Some('●'));
    assert_eq!(
        lines.iter().map(|l| l.matches('●').count()).sum::<usize>(),
        1
    );
    let pointer_row = lines.iter().position(|l| l.chars().nth(19) == Some('◂'));
    assert_eq!(pointer_row, Some(3));
}

#[test]
fn the_knob_has_a_contrasting_color() {
    let light = ColorPickerState::from_rgb(255, 255, 255);
    let (grid, _) = draw(&ColorPicker::new().view(&light), AREA, 20, 6);
    let square = ColorPicker::square_area(AREA);
    assert_eq!(style_at(&grid, square.x, square.y).fg, Color::Rgb(0, 0, 0));
    let dark = ColorPickerState::from_rgb(0, 0, 0);
    let (grid, _) = draw(&ColorPicker::new().view(&dark), AREA, 20, 6);
    assert_eq!(
        style_at(&grid, square.x, square.y + square.height - 1).fg,
        Color::Rgb(255, 255, 255)
    );
}

#[test]
fn the_readout_shows_swatch_hex_and_numbers() {
    let state = ColorPickerState::from_rgb(18, 171, 205);
    let picker = ColorPicker::new().view(&state);
    let lines = rows(&picker, AREA, 20, 6);
    assert_eq!(lines[5].trim_end(), "     #12ABCD H191");
    let (grid, _) = draw(&picker, AREA, 20, 6);
    for x in 0..4 {
        assert_eq!(style_at(&grid, x, 5).bg, Color::Rgb(18, 171, 205));
    }
    assert_ne!(style_at(&grid, 4, 5).bg, Color::Rgb(18, 171, 205));
}

#[test]
fn a_wide_picker_shows_value_too() {
    let state = ColorPickerState::from_rgb(18, 171, 205);
    let area = Rect::new(0, 0, 26, 6);
    let lines = rows(&ColorPicker::new().view(&state), area, 26, 6);
    assert_eq!(lines[5].trim_end(), "     #12ABCD H191 S91 V80");
}

#[test]
fn typing_shows_digits_and_underscores() {
    let mut state = ColorPickerState::from_rgb(18, 171, 205);
    state.set_focus(ColorPickerFocus::Hex);
    state.type_digit('f');
    state.type_digit('0');
    let lines = rows(&ColorPicker::new().view(&state), AREA, 20, 6);
    assert!(lines[5].contains("#F0____"), "{}", lines[5]);
    assert!(!lines[5].contains("#12ABCD"));
}

#[test]
fn only_the_focused_part_is_in_the_focus_ring_color() {
    let ring = token(Token::FocusRing);
    let text = token(Token::TextPrimary);
    let mut state = ColorPickerState::from_rgb(18, 171, 205);
    let picker = ColorPicker::new().state(WidgetState::Focused);
    let area = Rect::new(0, 0, 26, 6);
    for (focus, hex, hue, sat) in [
        (ColorPickerFocus::Square, text, text, ring),
        (ColorPickerFocus::Hue, text, ring, text),
        (ColorPickerFocus::Hex, ring, text, text),
    ] {
        state.set_focus(focus);
        let (grid, _) = draw(&picker.view(&state), area, 26, 6);
        assert_eq!(style_at(&grid, 5, 5).fg, hex, "{focus:?} hex");
        assert_eq!(style_at(&grid, 13, 5).fg, hue, "{focus:?} hue");
        assert_eq!(style_at(&grid, 18, 5).fg, sat, "{focus:?} saturation");
        assert_eq!(style_at(&grid, 22, 5).fg, sat, "{focus:?} value");
    }
}

#[test]
fn an_unfocused_picker_shows_no_focus_ring() {
    let ring = token(Token::FocusRing);
    let state = ColorPickerState::from_rgb(18, 171, 205);
    let (grid, _) = draw(&ColorPicker::new().view(&state), AREA, 20, 6);
    for x in 4..20 {
        assert_ne!(style_at(&grid, x, 5).fg, ring, "column {x}");
    }
}

#[test]
fn disabled_dims_every_cell_it_draws() {
    let state = ColorPickerState::from_rgb(18, 171, 205);
    let picker = ColorPicker::new().view(&state).state(WidgetState::Disabled);
    let (grid, clusters) = draw(&picker, AREA, 20, 6);
    let lines: Vec<String> = (0..6).map(|y| grid.row_display(y, &clusters)).collect();
    for (y, line) in lines.iter().enumerate() {
        for (x, ch) in line.chars().enumerate() {
            let y = u16::try_from(y).unwrap_or(0);
            let x = u16::try_from(x).unwrap_or(0);
            if ch != ' ' {
                assert!(style_at(&grid, x, y).attrs.dim(), "({x}, {y}) {ch}");
            }
        }
    }
}

#[test]
fn clicking_the_square_sets_saturation_and_value_from_the_corners() {
    let mut state = ColorPickerState::from_rgb(0, 0, 255);
    let square = ColorPicker::square_area(AREA);
    assert!(state.pointer(square.x, square.y, AREA));
    assert_eq!((state.saturation(), state.value()), (0.0, 1.0));
    assert!(state.pointer(
        square.x + square.width - 1,
        square.y + square.height - 1,
        AREA
    ));
    assert_eq!(state.saturation(), 1.0);
    assert!(
        (state.value() - 1.0 / 9.0).abs() < 1e-6,
        "{}",
        state.value()
    );
    assert_eq!(state.focus(), ColorPickerFocus::Square);
    assert_eq!(state.hue().round(), 240.0, "the hue is untouched");
}

#[test]
fn clicking_the_bar_sets_the_hue_and_focuses_it() {
    let mut state = ColorPickerState::new();
    let bar = ColorPicker::hue_area(AREA);
    assert!(state.pointer(bar.x, bar.y, AREA));
    assert_eq!(state.hue(), 0.0);
    assert_eq!(state.focus(), ColorPickerFocus::Hue);
    assert!(state.pointer(bar.x + 1, bar.y + 4, AREA));
    assert!(
        (state.hue() - 359.0 * 8.0 / 9.0).abs() < 1e-3,
        "{}",
        state.hue()
    );
    assert!(
        !state.pointer(bar.x + 2, bar.y, AREA),
        "the pointer column is not the bar"
    );
}

#[test]
fn clicking_the_hex_row_focuses_the_entry_without_changing_the_color() {
    let mut state = ColorPickerState::from_rgb(18, 171, 205);
    assert!(!state.pointer(6, 5, AREA));
    assert_eq!(state.focus(), ColorPickerFocus::Hex);
    assert_eq!(state.rgb(), (18, 171, 205));
}

#[test]
fn clicks_outside_the_picker_change_nothing() {
    let mut state = ColorPickerState::from_rgb(18, 171, 205);
    let before = state;
    assert!(!state.pointer(40, 40, AREA));
    assert!(
        !state.pointer(16, 0, AREA),
        "the gap between square and bar"
    );
    assert_eq!(state, before);
}

#[test]
fn pointer_positions_follow_the_area_origin() {
    let area = Rect::new(10, 4, 20, 6);
    let mut state = ColorPickerState::new();
    let square = ColorPicker::square_area(area);
    assert!(state.pointer(square.x, square.y, area));
    assert_eq!((state.saturation(), state.value()), (0.0, 1.0));
    assert!(!state.pointer(0, 0, area));
}

#[test]
fn empty_and_tiny_areas_draw_nothing_and_do_not_panic() {
    let state = ColorPickerState::new();
    let picker = ColorPicker::new().view(&state);
    for (w, h) in [(0, 0), (0, 5), (5, 0), (1, 1), (3, 2), (6, 1), (6, 2)] {
        let (_grid, _) = draw(&picker, Rect::new(0, 0, w, h), 24, 8);
    }
    let blank = rows(&picker, Rect::new(0, 0, 0, 0), 4, 2);
    assert_eq!(blank, ["    ", "    "]);
}

#[test]
fn it_measures_at_most_its_natural_size() {
    use lunatui_core::Size;
    let picker = ColorPicker::new();
    assert_eq!(picker.measure(Size::new(100, 100)), Size::new(24, 9));
    assert_eq!(picker.measure(Size::new(10, 3)), Size::new(10, 3));
}

#[test]
fn hsv_is_clamped_and_the_hue_wraps() {
    let state = ColorPickerState::from_hsv(-90.0, 2.0, -1.0);
    assert_eq!(state.hue(), 270.0);
    assert_eq!((state.saturation(), state.value()), (1.0, 0.0));
    let state = ColorPickerState::from_hsv(f32::NAN, f32::NAN, 0.5);
    assert_eq!(
        (state.hue(), state.saturation(), state.value()),
        (0.0, 0.0, 0.5)
    );
    let state = ColorPickerState::from_hsv(720.0, 1.0, 1.0);
    assert_eq!(state.rgb(), (255, 0, 0));
    let mut state = ColorPickerState::from_hsv(120.0, 1.0, 1.0);
    state.set_focus(ColorPickerFocus::Hex);
    state.type_digit('a');
    state.set_hsv(240.0, 1.0, 1.0);
    assert!(!state.is_editing());
    assert_eq!(state.rgb(), (0, 0, 255));
}

#[test]
fn a_hue_just_below_zero_wraps_to_zero_not_360() {
    let state = ColorPickerState::from_hsv(-1e-7, 1.0, 1.0);
    assert!(state.hue() < 360.0, "{}", state.hue());
    assert_eq!(state.rgb(), (255, 0, 0));
}

#[test]
fn the_knob_ink_counts_the_blue_channel() {
    let state = ColorPickerState::from_rgb(138, 138, 255);
    let (grid, _) = draw(&ColorPicker::new().view(&state), AREA, 20, 6);
    let square = ColorPicker::square_area(AREA);
    let knob_column = square.x + 7;
    assert_eq!(
        style_at(&grid, knob_column, square.y).fg,
        Color::Rgb(0, 0, 0)
    );
}
