// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use super::keys::{
    KeyEvent, encode_key, ground_key, key_from_arrow, key_from_kitty, key_from_ss3, key_from_tilde,
};
use super::mouse::{MouseEvent, encode_mouse, mouse_from_sgr, mouse_from_x10};
use super::paste::{CHUNK, Paste, encode_paste};

const MAX_PENDING: usize = 4096;
const PASTE_END: &[u8] = b"\x1b[201~";

/// Focus gained or lost.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    /// `CSI I`.
    In,
    /// `CSI O`.
    Out,
}

/// A decoded terminal event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// A key.
    Key(KeyEvent),
    /// A pointer event.
    Mouse(MouseEvent),
    /// Bracketed paste.
    Paste(Paste),
    /// Focus in or out.
    Focus(Focus),
}

impl Event {
    /// Bytes that [`Parser`] decodes back to this event.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Self::Key(key) => encode_key(key),
            Self::Mouse(mouse) => encode_mouse(mouse),
            Self::Paste(paste) => encode_paste(paste),
            Self::Focus(Focus::In) => b"\x1b[I".to_vec(),
            Self::Focus(Focus::Out) => b"\x1b[O".to_vec(),
        }
    }
}

/// Incremental VT input parser.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Parser {
    pending: Vec<u8>,
    paste: bool,
    pixels: bool,
}

impl Parser {
    /// An empty parser. Mouse coordinates are cells.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// When `enabled`, SGR coordinates are pixels (mode 1016).
    pub fn mouse_pixels(&mut self, enabled: bool) {
        self.pixels = enabled;
    }

    /// Bytes still held because a sequence is incomplete.
    #[must_use]
    pub fn pending(&self) -> usize {
        self.pending.len()
    }

    /// Feeds bytes and returns every event that completed.
    #[must_use]
    pub fn push(&mut self, bytes: &[u8]) -> Vec<Event> {
        let mut out = Vec::new();
        for byte in bytes {
            if self.pending.len() >= MAX_PENDING {
                self.force_one(&mut out);
            }
            self.pending.push(*byte);
            self.drain(&mut out);
        }
        out
    }

    /// Finishes a trailing escape or an open paste.
    #[must_use]
    pub fn flush(&mut self) -> Vec<Event> {
        let mut out = Vec::new();
        if self.paste {
            let rest = std::mem::take(&mut self.pending);
            self.paste = false;
            emit_chunks(&rest, &mut out);
            out.push(Event::Paste(Paste::End));
            return out;
        }
        self.drain(&mut out);
        if self.pending.first() == Some(&0x1b) {
            self.pending.remove(0);
            out.push(Event::Key(KeyEvent::press(super::keys::KeyCode::Esc)));
            self.drain(&mut out);
        }
        self.pending.clear();
        out
    }
}

/// Pushes `bytes` in chunks, then flushes. Fails when the hold buffer, a paste
/// slice, or the event count grows past its cap.
///
/// # Errors
///
/// Returns a static reason when a cap is broken.
pub fn fuzz_input_parser(bytes: &[u8]) -> Result<(), &'static str> {
    let mut parser = Parser::new();
    let mut events = 0usize;
    for chunk in bytes.chunks(64) {
        let produced = parser.push(chunk);
        if parser.pending() > MAX_PENDING {
            return Err("pending exceeds the cap");
        }
        if !bounded(&produced) {
            return Err("event exceeds the cap");
        }
        events = events.saturating_add(produced.len());
    }
    let tail = parser.flush();
    if !bounded(&tail) {
        return Err("event exceeds the cap");
    }
    events = events.saturating_add(tail.len());
    if parser.pending() != 0 {
        return Err("flush left pending bytes");
    }
    if events > bytes.len().saturating_mul(2).saturating_add(8) {
        return Err("event count grew faster than the input");
    }
    Ok(())
}

fn bounded(events: &[Event]) -> bool {
    events.iter().all(|event| match event {
        Event::Paste(Paste::Chunk(text)) => text.len() <= CHUNK,
        Event::Key(key) => key
            .text
            .as_ref()
            .is_none_or(|text| text.len() <= MAX_PENDING),
        Event::Mouse(_) | Event::Paste(_) | Event::Focus(_) => true,
    })
}

enum Action {
    Event(Event),
    PasteStart,
    Ignore,
}

enum Take {
    Hold,
    Drop,
    Key { len: usize, event: KeyEvent },
}

impl Parser {
    fn drain(&mut self, out: &mut Vec<Event>) {
        loop {
            if self.pending.is_empty() {
                return;
            }
            let progressed = if self.paste {
                self.drain_paste(out)
            } else {
                self.pull(out)
            };
            if !progressed {
                return;
            }
        }
    }

    fn drain_paste(&mut self, out: &mut Vec<Event>) -> bool {
        if let Some(pos) = find_end(&self.pending) {
            let text = self.pending[..pos].to_vec();
            self.pending.drain(..pos + PASTE_END.len());
            self.paste = false;
            emit_chunks(&text, out);
            out.push(Event::Paste(Paste::End));
            return true;
        }
        let keep = holdback(&self.pending);
        let safe_end = self.pending.len().saturating_sub(keep);
        if safe_end >= CHUNK {
            let take = valid_prefix_len(&self.pending[..safe_end]);
            if take == 0 {
                self.pending.drain(..1);
                return true;
            }
            let text = self.pending[..take].to_vec();
            self.pending.drain(..take);
            emit_chunks(&text, out);
            return true;
        }
        false
    }

    fn pull(&mut self, out: &mut Vec<Event>) -> bool {
        if self.pending.first() != Some(&0x1b) {
            return self.pull_ground(out);
        }
        if self.pending.len() == 1 {
            return false;
        }
        match self.pending[1] {
            b'[' => self.pull_csi(out),
            b'O' => self.pull_ss3(out),
            b']' | b'P' | b'X' | b'^' | b'_' => self.pull_string(),
            b'\\' => {
                self.pending.drain(..2);
                true
            }
            0x1b => {
                out.push(Event::Key(KeyEvent::press(super::keys::KeyCode::Esc)));
                self.pending.drain(..1);
                true
            }
            _ => self.pull_alt(out),
        }
    }

    fn pull_ground(&mut self, out: &mut Vec<Event>) -> bool {
        match take_ground(&self.pending) {
            Take::Hold => false,
            Take::Drop => {
                self.pending.drain(..1);
                true
            }
            Take::Key { len, event } => {
                out.push(Event::Key(event));
                self.pending.drain(..len);
                true
            }
        }
    }

    fn pull_alt(&mut self, out: &mut Vec<Event>) -> bool {
        match take_ground(&self.pending[1..]) {
            Take::Hold => false,
            Take::Drop => {
                out.push(Event::Key(KeyEvent::press(super::keys::KeyCode::Esc)));
                self.pending.drain(..1);
                true
            }
            Take::Key { len, event } => {
                let modifiers = event.modifiers.alt();
                let event = event.modifiers(modifiers);
                out.push(Event::Key(event));
                self.pending.drain(..=len);
                true
            }
        }
    }

    fn pull_ss3(&mut self, out: &mut Vec<Event>) -> bool {
        if self.pending.len() < 3 {
            return false;
        }
        if let Some(event) = key_from_ss3(self.pending[2]) {
            out.push(Event::Key(event));
        }
        self.pending.drain(..3);
        true
    }

    fn pull_string(&mut self) -> bool {
        let mut index = 2;
        while index < self.pending.len() {
            if self.pending[index] == 0x07 {
                self.pending.drain(..=index);
                return true;
            }
            if self.pending[index] == 0x1b && index + 1 == self.pending.len() {
                return false;
            }
            if self.pending[index] == 0x1b && self.pending[index + 1] == b'\\' {
                self.pending.drain(..=index + 1);
                return true;
            }
            index += 1;
        }
        false
    }

    fn pull_csi(&mut self, out: &mut Vec<Event>) -> bool {
        if self.pending.len() >= 3 && self.pending[2] == b'M' {
            if self.pending.len() < 6 {
                return false;
            }
            let raw = [self.pending[3], self.pending[4], self.pending[5]];
            if let Some(mouse) = mouse_from_x10(raw, self.pixels) {
                out.push(Event::Mouse(mouse));
            }
            self.pending.drain(..6);
            return true;
        }
        let mut index = 2;
        while index < self.pending.len() {
            let byte = self.pending[index];
            if (0x40..=0x7e).contains(&byte) {
                let body = self.pending[2..index].to_vec();
                let action = classify_csi(&body, byte, self.pixels);
                self.pending.drain(..=index);
                self.apply(action, out);
                return true;
            }
            if !(0x20..=0x3f).contains(&byte) {
                self.pending.drain(..=index);
                return true;
            }
            index += 1;
        }
        false
    }

    fn apply(&mut self, action: Action, out: &mut Vec<Event>) {
        match action {
            Action::Event(event) => out.push(event),
            Action::PasteStart => {
                self.paste = true;
                out.push(Event::Paste(Paste::Start));
            }
            Action::Ignore => {}
        }
    }

    fn force_one(&mut self, out: &mut Vec<Event>) {
        if self.pending.is_empty() {
            return;
        }
        let byte = self.pending.remove(0);
        if self.paste {
            emit_chunks(&[byte], out);
        } else if let Some(event) = ground_key(byte) {
            out.push(Event::Key(event));
        }
    }
}

fn classify_csi(body: &[u8], final_byte: u8, pixels: bool) -> Action {
    let Ok(text) = std::str::from_utf8(body) else {
        return Action::Ignore;
    };
    if final_byte == b'~' && text == "200" {
        return Action::PasteStart;
    }
    if final_byte == b'~' && text == "201" {
        return Action::Ignore;
    }
    if final_byte == b'u' && text.starts_with('?') {
        return Action::Ignore;
    }
    if final_byte == b'u' {
        return key_action(key_from_kitty(text));
    }
    if final_byte == b'~' {
        return key_action(key_from_tilde(text));
    }
    if text.starts_with('<') && (final_byte == b'M' || final_byte == b'm') {
        return mouse_action(mouse_from_sgr(&text[1..], final_byte == b'm', pixels));
    }
    if matches!(final_byte, b'A' | b'B' | b'C' | b'D' | b'H' | b'F' | b'Z') {
        return key_action(key_from_arrow(final_byte, text));
    }
    if text.is_empty() && final_byte == b'I' {
        return Action::Event(Event::Focus(Focus::In));
    }
    if text.is_empty() && final_byte == b'O' {
        return Action::Event(Event::Focus(Focus::Out));
    }
    Action::Ignore
}

fn key_action(key: Option<KeyEvent>) -> Action {
    match key {
        Some(event) => Action::Event(Event::Key(event)),
        None => Action::Ignore,
    }
}

fn mouse_action(mouse: Option<MouseEvent>) -> Action {
    match mouse {
        Some(event) => Action::Event(Event::Mouse(event)),
        None => Action::Ignore,
    }
}

fn take_ground(bytes: &[u8]) -> Take {
    let Some(lead) = bytes.first().copied() else {
        return Take::Hold;
    };
    if lead < 0x80 {
        return match ground_key(lead) {
            Some(event) => Take::Key { len: 1, event },
            None => Take::Drop,
        };
    }
    let Some(width) = utf8_width(lead) else {
        return Take::Drop;
    };
    if bytes.len() < width {
        if bytes[1..]
            .iter()
            .all(|byte| byte & 0b1100_0000 == 0b1000_0000)
        {
            Take::Hold
        } else {
            Take::Drop
        }
    } else if let Ok(text) = std::str::from_utf8(&bytes[..width]) {
        match text.chars().next() {
            Some(ch) => Take::Key {
                len: width,
                event: KeyEvent::press(super::keys::KeyCode::Char(ch)),
            },
            None => Take::Drop,
        }
    } else {
        Take::Drop
    }
}

fn utf8_width(lead: u8) -> Option<usize> {
    match lead {
        0xC2..=0xDF => Some(2),
        0xE0..=0xEF => Some(3),
        0xF0..=0xF4 => Some(4),
        _ => None,
    }
}

fn find_end(bytes: &[u8]) -> Option<usize> {
    bytes
        .windows(PASTE_END.len())
        .position(|window| window == PASTE_END)
}

fn holdback(bytes: &[u8]) -> usize {
    let max = bytes.len().min(PASTE_END.len() - 1);
    for len in (1..=max).rev() {
        if PASTE_END.starts_with(&bytes[bytes.len() - len..]) {
            return len;
        }
    }
    0
}

fn valid_prefix_len(bytes: &[u8]) -> usize {
    let limit = bytes.len().min(CHUNK);
    match std::str::from_utf8(&bytes[..limit]) {
        Ok(_) => limit,
        Err(error) => error.valid_up_to(),
    }
}

fn emit_chunks(bytes: &[u8], out: &mut Vec<Event>) {
    let mut start = 0;
    while start < bytes.len() {
        let take = valid_prefix_len(&bytes[start..]);
        if take == 0 {
            start += 1;
            continue;
        }
        let Ok(text) = std::str::from_utf8(&bytes[start..start + take]) else {
            start += 1;
            continue;
        };
        if !text.is_empty() {
            out.push(Event::Paste(Paste::Chunk(String::from(text))));
        }
        start += take;
    }
}
