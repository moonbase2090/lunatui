// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style, UnderlineStyle};
use lunatui_theme::{Theme, Token};
use lunatui_widgets::{
    Block, BorderType, Button, ColorPicker, ColorPickerFocus, ColorPickerState, LineGauge,
    Paragraph, RenderCx, StatusBar, Widget, WidgetState,
};

const START: (u8, u8, u8) = (58, 124, 214);
const PICKER: Rect = Rect::new(3, 4, 40, 18);

const DRAG_FROM: (u16, u16) = (6, 12);
const DRAG_TO: (u16, u16) = (31, 4);
const DRAG_START: u32 = 16;
const DRAG_END: u32 = 56;
const TO_HUE: u32 = 66;
const HUE_START: u32 = 72;
const HUE_END: u32 = 132;
const TO_HEX: u32 = 140;
const TYPE_START: u32 = 150;
const TYPE_STEP: u32 = 7;
const COMMIT: u32 = 200;
const TO_SQUARE: u32 = 214;
const FINAL_START: u32 = 220;
const FINAL_END: u32 = 232;

const DIGITS: [char; 6] = ['f', '0', '8', 'c', '0', '0'];

const SAVED: [(&str, &str); 5] = [
    ("Brand", "#6C5CE7"),
    ("Success", "#3FB68B"),
    ("Warning", "#E5A93C"),
    ("Danger", "#D6564F"),
    ("Muted", "#7B8794"),
];

struct App {
    picker: ColorPickerState,
    key: &'static str,
    note: &'static str,
    committed: bool,
}

fn replay(tick: u32) -> App {
    let mut app = App {
        picker: ColorPickerState::from_rgb(START.0, START.1, START.2),
        key: "",
        note: "Pick a color for the accent",
        committed: false,
    };
    let area = PICKER;
    for t in 0..=tick {
        match t {
            DRAG_START..=DRAG_END => {
                let spent = t - DRAG_START;
                let span = DRAG_END - DRAG_START;
                let along = |from: u16, to: u16| {
                    let from = i32::from(from);
                    let to = i32::from(to);
                    let at = from
                        + (to - from) * i32::try_from(spent).unwrap_or(0)
                            / i32::try_from(span).unwrap_or(1);
                    u16::try_from(at).unwrap_or(0)
                };
                let square = ColorPicker::square_area(area);
                app.picker.pointer(
                    square.x + along(DRAG_FROM.0, DRAG_TO.0),
                    square.y + along(DRAG_FROM.1, DRAG_TO.1),
                    area,
                );
                app.key = "drag";
                app.note = "Drag in the square for saturation and value";
            }
            TO_HUE => {
                app.picker.next_focus();
                app.key = "Tab";
                app.note = "Tab moves to the hue bar";
            }
            HUE_START..=HUE_END if t.is_multiple_of(5) => {
                app.picker.nudge(12, 0);
                app.key = "→";
                app.note = "Arrow keys turn the hue";
            }
            TO_HEX => {
                app.picker.next_focus();
                app.key = "Tab";
                app.note = "Tab moves to the hex entry";
            }
            COMMIT => {
                app.committed = app.picker.commit_hex();
                app.key = "Enter";
                app.note = "Enter applies the typed color";
            }
            TO_SQUARE => {
                app.picker.next_focus();
                app.key = "Tab";
                app.note = "Tab comes back round to the square";
            }
            FINAL_START..=FINAL_END if t.is_multiple_of(3) => {
                app.picker.nudge(-1, 2);
                app.key = "↓";
                app.note = "Arrow keys step by one percent";
            }
            _ => {}
        }
        if (TYPE_START..COMMIT).contains(&t) && (t - TYPE_START).is_multiple_of(TYPE_STEP) {
            let index = usize::try_from((t - TYPE_START) / TYPE_STEP).unwrap_or(usize::MAX);
            if let Some(digit) = DIGITS.get(index) {
                app.picker.type_digit(*digit);
                app.key = "type";
                app.note = "Type a hex color, three or six digits";
            }
        }
    }
    app
}

pub(super) fn render(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let text = theme.resolve(Token::TextPrimary);
    let fill = Style::new().fg(text).bg(theme.resolve(Token::Surface0));
    let raised = Style::new().fg(text).bg(theme.resolve(Token::Surface1));
    let accent = theme.resolve(Token::Accent);
    *grid = Grid::new(grid.width(), grid.height());
    let mut cx = RenderCx::new(grid, clusters, theme);
    Block::new().style(fill).render(area, &mut cx);
    if area.width < 100 || area.height < 30 {
        return;
    }
    let app = replay(tick);
    let picked = app.picker.color();
    Paragraph::new(format!(
        " acme/web · theme editor{:>width$}",
        "accent color ",
        width = usize::from(area.width) - 24
    ))
    .style(raised.fg(accent).with_attrs(Attrs::new().set_bold(true)))
    .render(Rect::new(area.x, area.y, area.width, 1), &mut cx);
    let editor = Rect::new(area.x + 1, area.y + 2, 46, 24);
    let preview = Rect::new(
        editor.x + editor.width + 1,
        editor.y,
        area.width - editor.width - 3,
        editor.height,
    );
    pane(&mut cx, editor, " Accent color ", fill, accent);
    ColorPicker::new()
        .view(&app.picker)
        .state(WidgetState::Focused)
        .render(
            Rect::new(
                area.x + PICKER.x,
                area.y + PICKER.y,
                PICKER.width,
                PICKER.height,
            ),
            &mut cx,
        );
    editor_notes(&mut cx, editor, &app, fill);
    pane(&mut cx, preview, " Preview ", fill, accent);
    preview_pane(&mut cx, preview, picked, fill, theme);
    let saved = Rect::new(
        area.x + 1,
        editor.y + editor.height + 1,
        area.width - 2,
        area.height - editor.height - 4,
    );
    saved_colors(&mut cx, saved, &app, fill, accent);
    let dim = raised.with_attrs(Attrs::new().set_dim(true));
    StatusBar::new("")
        .style(raised)
        .left([
            (
                if app.key.is_empty() {
                    String::new()
                } else {
                    format!(" key {} ", app.key)
                },
                raised.fg(theme.resolve(Token::FocusRing)),
            ),
            (format!(" {} ", app.note), raised.fg(accent)),
        ])
        .right([(
            String::from(" Tab next part  ←↑↓→ nudge  click or drag "),
            dim,
        )])
        .render(
            Rect::new(area.x, area.y + area.height - 1, area.width, 1),
            &mut cx,
        );
}

fn pane(cx: &mut RenderCx<'_>, area: Rect, title: &str, fill: Style, accent: Color) {
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(title)
        .style(fill)
        .border_style(dim)
        .title_style(fill.fg(accent))
        .render(area, cx);
}

fn editor_notes(cx: &mut RenderCx<'_>, editor: Rect, app: &App, fill: Style) {
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let part = match app.picker.focus() {
        ColorPickerFocus::Square => "saturation and value",
        ColorPickerFocus::Hue => "hue",
        ColorPickerFocus::Hex => "hex entry",
    };
    Paragraph::new(format!("Editing {part}"))
        .style(fill)
        .render(
            Rect::new(editor.x + 2, editor.y + 1, editor.width - 4, 1),
            cx,
        );
    Paragraph::new("Colors the buttons, gauges and links.")
        .style(dim)
        .render(
            Rect::new(editor.x + 2, editor.y + 22, editor.width - 4, 1),
            cx,
        );
}

fn preview_pane(cx: &mut RenderCx<'_>, area: Rect, picked: Color, fill: Style, theme: &Theme) {
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let surface = theme.resolve(Token::Surface0);
    let x = area.x + 3;
    let width = area.width - 6;
    Paragraph::new("Billing overview")
        .style(fill.fg(picked).with_attrs(Attrs::new().set_bold(true)))
        .render(Rect::new(x, area.y + 2, width, 1), cx);
    Paragraph::new("Invoices, usage and payment methods for the current period")
        .style(dim)
        .render(Rect::new(x, area.y + 3, width, 1), cx);
    Button::new("Save changes")
        .style(
            Style::new()
                .fg(surface)
                .bg(picked)
                .with_attrs(Attrs::new().set_bold(true)),
        )
        .render(Rect::new(x, area.y + 5, 18, 3), cx);
    Button::new("Cancel").render(Rect::new(x + 20, area.y + 5, 12, 3), cx);
    for (row, (label, fraction)) in
        (0_u16..).zip([("Storage", 0.72), ("Seats", 0.45), ("API calls", 0.88)])
    {
        let y = area.y + 9 + row * 2;
        Paragraph::new(label)
            .style(fill)
            .render(Rect::new(x, y, 12, 1), cx);
        LineGauge::new(fraction)
            .filled_style(fill.fg(picked))
            .unfilled_style(dim)
            .render(Rect::new(x + 13, y, width - 20, 1), cx);
        Paragraph::new(format!("{:>3}%", (fraction * 100.0).round()))
            .style(fill.fg(picked))
            .render(Rect::new(x + width - 5, y, 5, 1), cx);
    }
    Paragraph::new("View all invoices →")
        .style(
            fill.fg(picked)
                .with_attrs(Attrs::new().set_underline(UnderlineStyle::Single)),
        )
        .render(Rect::new(x, area.y + 16, width, 1), cx);
}

fn saved_colors(cx: &mut RenderCx<'_>, area: Rect, app: &App, fill: Style, accent: Color) {
    pane(cx, area, " Saved colors ", fill, accent);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let inner = Rect::new(area.x + 2, area.y + 1, area.width - 4, area.height - 2);
    let current = app.picker.hex();
    let current = core::str::from_utf8(&current).unwrap_or("#000000");
    let rows = [("Accent", current)].into_iter().chain(SAVED);
    for (row, (name, hex)) in (0_u16..).zip(rows) {
        if row >= inner.height {
            break;
        }
        let color = parse(hex);
        Paragraph::new("    ")
            .style(Style::new().bg(color))
            .render(Rect::new(inner.x, inner.y + row, 4, 1), cx);
        Paragraph::new(name)
            .style(if row == 0 { fill.fg(color) } else { fill })
            .render(Rect::new(inner.x + 6, inner.y + row, 12, 1), cx);
        Paragraph::new(hex)
            .style(dim)
            .render(Rect::new(inner.x + 19, inner.y + row, 8, 1), cx);
    }
}

fn parse(hex: &str) -> Color {
    let byte = |at: usize| {
        hex.get(at..at + 2)
            .and_then(|digits| u8::from_str_radix(digits, 16).ok())
            .unwrap_or(0)
    };
    Color::Rgb(byte(1), byte(3), byte(5))
}
