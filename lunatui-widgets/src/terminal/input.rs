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
    if let Some(sequence) = SEQUENCES
        .iter()
        .find_map(|(code, sequence)| (*code == key.code).then_some(sequence))
    {
        return Some(sequence_bytes(screen, sequence, value));
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
enum Sequence {
    Cursor(char),
    Function(char),
    Tilde(u8),
}
const SEQUENCES: [(KeyCode, Sequence); 22] = [
    (KeyCode::Up, Sequence::Cursor('A')),
    (KeyCode::Down, Sequence::Cursor('B')),
    (KeyCode::Right, Sequence::Cursor('C')),
    (KeyCode::Left, Sequence::Cursor('D')),
    (KeyCode::Home, Sequence::Cursor('H')),
    (KeyCode::End, Sequence::Cursor('F')),
    (KeyCode::F(1), Sequence::Function('P')),
    (KeyCode::F(2), Sequence::Function('Q')),
    (KeyCode::F(3), Sequence::Function('R')),
    (KeyCode::F(4), Sequence::Function('S')),
    (KeyCode::Insert, Sequence::Tilde(2)),
    (KeyCode::Delete, Sequence::Tilde(3)),
    (KeyCode::PageUp, Sequence::Tilde(5)),
    (KeyCode::PageDown, Sequence::Tilde(6)),
    (KeyCode::F(5), Sequence::Tilde(15)),
    (KeyCode::F(6), Sequence::Tilde(17)),
    (KeyCode::F(7), Sequence::Tilde(18)),
    (KeyCode::F(8), Sequence::Tilde(19)),
    (KeyCode::F(9), Sequence::Tilde(20)),
    (KeyCode::F(10), Sequence::Tilde(21)),
    (KeyCode::F(11), Sequence::Tilde(23)),
    (KeyCode::F(12), Sequence::Tilde(24)),
];
fn sequence_bytes(screen: &TerminalScreen, sequence: &Sequence, value: u8) -> Vec<u8> {
    match sequence {
        Sequence::Cursor(tail) => {
            if value > 1 {
                format!("\x1b[1;{value}{tail}")
            } else if screen.application_cursor() {
                format!("\x1bO{tail}")
            } else {
                format!("\x1b[{tail}")
            }
        }
        Sequence::Function(tail) => {
            if value == 1 {
                format!("\x1bO{tail}")
            } else {
                format!("\x1b[1;{value}{tail}")
            }
        }
        Sequence::Tilde(number) => {
            if value == 1 {
                format!("\x1b[{number}~")
            } else {
                format!("\x1b[{number};{value}~")
            }
        }
    }
    .into_bytes()
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
