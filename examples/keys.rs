// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::io::{self, IsTerminal, Read, Write};

use lunatui_core::width;
use lunatui_term::{
    Event, Focus, KeyCode, KeyEvent, KeyKind, MouseButton, MouseKind, MouseUnit, Parser, Paste,
    Scroll,
};

fn main() -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(picture(&demo_bytes()).as_bytes())?;
    stdout.flush()
}

fn demo_bytes() -> Vec<u8> {
    if io::stdin().is_terminal() {
        return script().to_vec();
    }
    let mut buf = Vec::new();
    if io::stdin().read_to_end(&mut buf).is_err() || buf.is_empty() {
        return script().to_vec();
    }
    buf
}

fn script() -> &'static [u8] {
    b"a\x1b[A\x1b[99;5u\x1b[<0;2;3M\x1b[200~hi\x1b[201~\x1b[I"
}

fn picture(bytes: &[u8]) -> String {
    let mut parser = Parser::new();
    let mut events = parser.push(bytes);
    events.extend(parser.flush());
    let rows: Vec<String> = events.iter().map(label).collect();
    draw(&rows)
}

fn label(event: &Event) -> String {
    match event {
        Event::Key(key) => format!("key {}", key_label(key)),
        Event::Mouse(mouse) => format!("mouse {}", mouse_label(mouse)),
        Event::Paste(Paste::Start) => String::from("paste start"),
        Event::Paste(Paste::End) => String::from("paste end"),
        Event::Paste(Paste::Chunk(text)) => format!("paste {text}"),
        Event::Focus(Focus::In) => String::from("focus in"),
        Event::Focus(Focus::Out) => String::from("focus out"),
    }
}

fn key_label(key: &KeyEvent) -> String {
    let mut text = code_name(key.code);
    if key.keypad {
        text.push_str(" keypad");
    }
    if key.modifiers.is_control() {
        text.push_str(" ctrl");
    }
    if key.modifiers.is_alt() {
        text.push_str(" alt");
    }
    if key.modifiers.is_shift() {
        text.push_str(" shift");
    }
    if key.modifiers.is_super() {
        text.push_str(" super");
    }
    if key.modifiers.is_hyper() {
        text.push_str(" hyper");
    }
    if key.modifiers.is_meta() {
        text.push_str(" meta");
    }
    match key.kind {
        KeyKind::Press => {}
        KeyKind::Repeat => text.push_str(" repeat"),
        KeyKind::Release => text.push_str(" release"),
    }
    if let Some(extra) = &key.text {
        text.push(' ');
        text.push_str(extra);
    }
    text
}

fn code_name(code: KeyCode) -> String {
    match code {
        KeyCode::Char(ch) => ch.to_string(),
        KeyCode::Enter => String::from("enter"),
        KeyCode::Esc => String::from("esc"),
        KeyCode::Backspace => String::from("backspace"),
        KeyCode::Tab => String::from("tab"),
        KeyCode::Up => String::from("up"),
        KeyCode::Down => String::from("down"),
        KeyCode::Left => String::from("left"),
        KeyCode::Right => String::from("right"),
        KeyCode::Home => String::from("home"),
        KeyCode::End => String::from("end"),
        KeyCode::PageUp => String::from("pageup"),
        KeyCode::PageDown => String::from("pagedown"),
        KeyCode::Insert => String::from("insert"),
        KeyCode::Delete => String::from("delete"),
        KeyCode::F(number) => format!("f{number}"),
        KeyCode::CapsLock => String::from("capslock"),
        KeyCode::ScrollLock => String::from("scrolllock"),
        KeyCode::NumLock => String::from("numlock"),
        KeyCode::PrintScreen => String::from("printscreen"),
        KeyCode::Pause => String::from("pause"),
        KeyCode::Menu => String::from("menu"),
        KeyCode::Other(number) => format!("other {number}"),
    }
}

fn mouse_label(mouse: &lunatui_term::MouseEvent) -> String {
    let action = match mouse.kind {
        MouseKind::Down(button) => format!("down {}", button_name(button)),
        MouseKind::Up(button) => format!("up {}", button_name(button)),
        MouseKind::Drag(button) => format!("drag {}", button_name(button)),
        MouseKind::Move => String::from("move"),
        MouseKind::Scroll(Scroll::Up) => String::from("wheel up"),
        MouseKind::Scroll(Scroll::Down) => String::from("wheel down"),
        MouseKind::Scroll(Scroll::Left) => String::from("wheel left"),
        MouseKind::Scroll(Scroll::Right) => String::from("wheel right"),
    };
    let unit = match mouse.unit {
        MouseUnit::Cells => "",
        MouseUnit::Pixels => " px",
    };
    format!("{action} {},{}{unit}", mouse.column, mouse.row)
}

fn button_name(button: MouseButton) -> String {
    match button {
        MouseButton::Left => String::from("left"),
        MouseButton::Middle => String::from("middle"),
        MouseButton::Right => String::from("right"),
        MouseButton::Extra(number) => format!("button {number}"),
    }
}

fn draw(rows: &[String]) -> String {
    let inner = rows.iter().map(|row| width(row)).max().unwrap_or(0);
    let mut out = String::new();
    push_border(&mut out, '┌', '┐', inner, true);
    for row in rows {
        let pad = inner - width(row);
        out.push('│');
        out.push(' ');
        out.push_str(row);
        out.push_str(&" ".repeat(pad));
        out.push(' ');
        out.push('│');
        out.push('\n');
    }
    push_border(&mut out, '└', '┘', inner, false);
    out
}

fn push_border(out: &mut String, left: char, right: char, inner: usize, titled: bool) {
    let span = inner + 2;
    out.push(left);
    if titled {
        let label = " keys ";
        let label_width = width(label);
        out.push_str(label);
        if span > label_width {
            out.push_str(&"─".repeat(span - label_width));
        }
    } else {
        out.push_str(&"─".repeat(span));
    }
    out.push(right);
    out.push('\n');
}

#[cfg(test)]
mod tests {
    use super::{picture, script};

    #[test]
    fn the_script_shows_key_mouse_paste_and_focus() {
        let frame = picture(script());
        assert!(frame.contains("key a"));
        assert!(frame.contains("key up"));
        assert!(frame.contains("key c ctrl"));
        assert!(frame.contains("mouse down left 1,2"));
        assert!(frame.contains("paste start"));
        assert!(frame.contains("paste hi"));
        assert!(frame.contains("paste end"));
        assert!(frame.contains("focus in"));
        assert_eq!(frame, KEYS_BOX);
    }

    const KEYS_BOX: &str = "┌ keys ───────────────┐\n│ key a               │\n│ key up              │\n│ key c ctrl          │\n│ mouse down left 1,2 │\n│ paste start         │\n│ paste hi            │\n│ paste end           │\n│ focus in            │\n└─────────────────────┘\n";
}
