// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use super::TerminalScreen;
use lunatui_term::{Event, KeyCode, KeyEvent, KeyKind, Paste};

pub(super) fn encode(screen: &TerminalScreen, event: &Event) -> Option<Vec<u8>> {
    match event {
        Event::Key(key) if key.kind != KeyKind::Release => key_bytes(screen, key),
        Event::Paste(Paste::Chunk(text)) => Some(text.as_bytes().to_vec()),
        Event::Paste(Paste::Start) if screen.bracketed_paste() => Some(b"\x1b[200~".to_vec()),
        Event::Paste(Paste::End) if screen.bracketed_paste() => Some(b"\x1b[201~".to_vec()),
        _ => None,
    }
}
fn key_bytes(screen: &TerminalScreen, key: &KeyEvent) -> Option<Vec<u8>> {
    let modifiers = key.modifiers;
    if modifiers.is_super() || modifiers.is_hyper() || modifiers.is_meta() {
        return None;
    }
    let value = 1
        + u8::from(modifiers.is_shift())
        + 2 * u8::from(modifiers.is_alt())
        + 4 * u8::from(modifiers.is_control());
    if let Some(tail) = match key.code {
        KeyCode::Up => Some('A'),
        KeyCode::Down => Some('B'),
        KeyCode::Right => Some('C'),
        KeyCode::Left => Some('D'),
        KeyCode::Home => Some('H'),
        KeyCode::End => Some('F'),
        _ => None,
    } {
        return Some(
            if value > 1 {
                format!("\x1b[1;{value}{tail}")
            } else if screen.application_cursor() {
                format!("\x1bO{tail}")
            } else {
                format!("\x1b[{tail}")
            }
            .into_bytes(),
        );
    }
    if let KeyCode::F(number @ 1..=4) = key.code {
        let tail = char::from(b'P' + number - 1);
        return Some(
            if value == 1 {
                format!("\x1bO{tail}")
            } else {
                format!("\x1b[1;{value}{tail}")
            }
            .into_bytes(),
        );
    }
    let number = match key.code {
        KeyCode::Insert => Some(2),
        KeyCode::Delete => Some(3),
        KeyCode::PageUp => Some(5),
        KeyCode::PageDown => Some(6),
        KeyCode::F(5) => Some(15),
        KeyCode::F(6) => Some(17),
        KeyCode::F(7) => Some(18),
        KeyCode::F(8) => Some(19),
        KeyCode::F(9) => Some(20),
        KeyCode::F(10) => Some(21),
        KeyCode::F(11) => Some(23),
        KeyCode::F(12) => Some(24),
        _ => None,
    };
    if let Some(number) = number {
        return Some(
            if value == 1 {
                format!("\x1b[{number}~")
            } else {
                format!("\x1b[{number};{value}~")
            }
            .into_bytes(),
        );
    }
    let mut bytes = match key.code {
        KeyCode::Char(ch) if modifiers.is_control() => vec![control(ch)?],
        KeyCode::Char(ch) => key
            .text
            .clone()
            .unwrap_or_else(|| ch.to_string())
            .into_bytes(),
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Tab if modifiers.is_shift() => b"\x1b[Z".to_vec(),
        KeyCode::Tab => vec![b'\t'],
        _ => return None,
    };
    if modifiers.is_alt() {
        bytes.insert(0, 0x1b);
    }
    Some(bytes)
}
fn control(ch: char) -> Option<u8> {
    match ch {
        ' ' | '@' => Some(0),
        '?' => Some(0x7f),
        ch if ch.is_ascii() && ('A'..='_').contains(&ch.to_ascii_uppercase()) => {
            Some(u8::try_from(u32::from(ch.to_ascii_uppercase())).ok()? & 0x1f)
        }
        _ => None,
    }
}
