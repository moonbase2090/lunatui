// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

/// Shift, alt, control and the other kitty modifier bits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    bits: u16,
}

impl Modifiers {
    /// No modifiers.
    #[must_use]
    pub const fn new() -> Self {
        Self { bits: 0 }
    }

    /// Sets shift.
    #[must_use]
    pub const fn shift(self) -> Self {
        self.with(1)
    }

    /// Sets alt.
    #[must_use]
    pub const fn alt(self) -> Self {
        self.with(2)
    }

    /// Sets control.
    #[must_use]
    pub const fn control(self) -> Self {
        self.with(4)
    }

    /// Sets super.
    #[must_use]
    pub const fn super_key(self) -> Self {
        self.with(8)
    }

    /// Sets hyper.
    #[must_use]
    pub const fn hyper(self) -> Self {
        self.with(16)
    }

    /// Sets meta.
    #[must_use]
    pub const fn meta(self) -> Self {
        self.with(32)
    }

    /// Sets caps lock.
    #[must_use]
    pub const fn caps_lock(self) -> Self {
        self.with(64)
    }

    /// Sets num lock.
    #[must_use]
    pub const fn num_lock(self) -> Self {
        self.with(128)
    }

    /// Whether shift is set.
    #[must_use]
    pub const fn is_shift(self) -> bool {
        self.bits & 1 != 0
    }

    /// Whether alt is set.
    #[must_use]
    pub const fn is_alt(self) -> bool {
        self.bits & 2 != 0
    }

    /// Whether control is set.
    #[must_use]
    pub const fn is_control(self) -> bool {
        self.bits & 4 != 0
    }

    /// Whether super is set.
    #[must_use]
    pub const fn is_super(self) -> bool {
        self.bits & 8 != 0
    }

    /// Whether hyper is set.
    #[must_use]
    pub const fn is_hyper(self) -> bool {
        self.bits & 16 != 0
    }

    /// Whether meta is set.
    #[must_use]
    pub const fn is_meta(self) -> bool {
        self.bits & 32 != 0
    }

    /// Whether caps lock is set.
    #[must_use]
    pub const fn is_caps_lock(self) -> bool {
        self.bits & 64 != 0
    }

    /// Whether num lock is set.
    #[must_use]
    pub const fn is_num_lock(self) -> bool {
        self.bits & 128 != 0
    }

    pub(crate) fn from_kitty(value: u32) -> Self {
        let Ok(bits) = u16::try_from(value.saturating_sub(1) & 0xff) else {
            return Self::new();
        };
        Self { bits }
    }

    pub(crate) fn to_kitty(self) -> u32 {
        u32::from(self.bits) + 1
    }

    const fn with(self, bit: u16) -> Self {
        Self {
            bits: self.bits | bit,
        }
    }
}

/// A key identity after legacy and kitty sequences are normalized.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyCode {
    /// A Unicode scalar.
    Char(char),
    /// Enter or return.
    Enter,
    /// Escape.
    Esc,
    /// Backspace.
    Backspace,
    /// Tab.
    Tab,
    /// Up arrow.
    Up,
    /// Down arrow.
    Down,
    /// Left arrow.
    Left,
    /// Right arrow.
    Right,
    /// Home.
    Home,
    /// End.
    End,
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,
    /// Insert.
    Insert,
    /// Delete.
    Delete,
    /// Function key `n`, starting at 1.
    F(u8),
    /// Caps lock.
    CapsLock,
    /// Scroll lock.
    ScrollLock,
    /// Num lock.
    NumLock,
    /// Print screen.
    PrintScreen,
    /// Pause.
    Pause,
    /// Menu.
    Menu,
    /// A kitty code with no named key.
    Other(u32),
}

/// Press, repeat or release.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyKind {
    /// The key went down.
    #[default]
    Press,
    /// The key is repeating.
    Repeat,
    /// The key went up.
    Release,
}

/// One decoded key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyEvent {
    /// Key identity.
    pub code: KeyCode,
    /// Modifier bits.
    pub modifiers: Modifiers,
    /// Press, repeat or release.
    pub kind: KeyKind,
    /// Kitty alternate keys, packed from the first slot.
    pub alternates: [Option<KeyCode>; 2],
    /// Associated text, when the terminal sent it.
    pub text: Option<String>,
    /// The code came from the numeric keypad block.
    pub keypad: bool,
}

impl KeyEvent {
    /// A press with no modifiers, alternates or text.
    #[must_use]
    pub const fn press(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: Modifiers::new(),
            kind: KeyKind::Press,
            alternates: [None, None],
            text: None,
            keypad: false,
        }
    }

    /// Replaces the modifiers.
    #[must_use]
    pub const fn modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = modifiers;
        self
    }

    /// Replaces the event kind.
    #[must_use]
    pub const fn kind(mut self, kind: KeyKind) -> Self {
        self.kind = kind;
        self
    }

    /// Records a keypad origin.
    #[must_use]
    pub const fn keypad(mut self, keypad: bool) -> Self {
        self.keypad = keypad;
        self
    }

    /// Stores one alternate key, filling the first empty slot.
    #[must_use]
    pub const fn alternate(mut self, code: KeyCode) -> Self {
        if self.alternates[0].is_none() {
            self.alternates[0] = Some(code);
        } else {
            self.alternates[1] = Some(code);
        }
        self
    }

    /// Stores associated text.
    #[must_use]
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }
}

pub(crate) fn decimal(text: &str) -> Option<u32> {
    if text.is_empty() {
        return None;
    }
    let mut value = 0u32;
    for byte in text.bytes() {
        let digit = byte.checked_sub(b'0')?;
        if digit > 9 {
            return None;
        }
        value = value.checked_mul(10)?.checked_add(u32::from(digit))?;
    }
    Some(value)
}

pub(crate) fn ground_key(byte: u8) -> Option<KeyEvent> {
    let event = match byte {
        0x00 => KeyEvent::press(KeyCode::Char(' ')).modifiers(Modifiers::new().control()),
        0x08 | 0x7f => KeyEvent::press(KeyCode::Backspace),
        0x09 => KeyEvent::press(KeyCode::Tab),
        0x0a | 0x0d => KeyEvent::press(KeyCode::Enter),
        0x01..=0x1a => {
            let letter = b'a' + (byte - 1);
            KeyEvent::press(KeyCode::Char(char::from(letter))).modifiers(Modifiers::new().control())
        }
        0x1c => KeyEvent::press(KeyCode::Char('\\')).modifiers(Modifiers::new().control()),
        0x1d => KeyEvent::press(KeyCode::Char(']')).modifiers(Modifiers::new().control()),
        0x1e => KeyEvent::press(KeyCode::Char('^')).modifiers(Modifiers::new().control()),
        0x1f => KeyEvent::press(KeyCode::Char('_')).modifiers(Modifiers::new().control()),
        0x20..=0x7e => KeyEvent::press(KeyCode::Char(char::from(byte))),
        _ => return None,
    };
    Some(event)
}

pub(crate) fn key_from_ss3(byte: u8) -> Option<KeyEvent> {
    let code = match byte {
        b'A' => KeyCode::Up,
        b'B' => KeyCode::Down,
        b'C' => KeyCode::Right,
        b'D' => KeyCode::Left,
        b'H' => KeyCode::Home,
        b'F' => KeyCode::End,
        b'P' => KeyCode::F(1),
        b'Q' => KeyCode::F(2),
        b'R' => KeyCode::F(3),
        b'S' => KeyCode::F(4),
        _ => return None,
    };
    Some(KeyEvent::press(code))
}

pub(crate) fn key_from_arrow(final_byte: u8, body: &str) -> Option<KeyEvent> {
    let code = match final_byte {
        b'A' => KeyCode::Up,
        b'B' => KeyCode::Down,
        b'C' => KeyCode::Right,
        b'D' => KeyCode::Left,
        b'H' => KeyCode::Home,
        b'F' => KeyCode::End,
        b'Z' => KeyCode::Tab,
        _ => return None,
    };
    let (modifiers, kind) = mods_from_body(body)?;
    let modifiers = if final_byte == b'Z' {
        modifiers.shift()
    } else {
        modifiers
    };
    Some(KeyEvent::press(code).modifiers(modifiers).kind(kind))
}

pub(crate) fn key_from_tilde(body: &str) -> Option<KeyEvent> {
    let mut parts = body.split(';');
    let number = decimal(parts.next()?)?;
    let (modifiers, kind) = match parts.next() {
        None => (Modifiers::new(), KeyKind::Press),
        Some(field) => parse_mods_kind(field)?,
    };
    let code = tilde_code(number)?;
    Some(KeyEvent::press(code).modifiers(modifiers).kind(kind))
}

pub(crate) fn key_from_kitty(body: &str) -> Option<KeyEvent> {
    let mut parts = body.split(';');
    let mut codes = parts.next()?.split(':');
    let primary = decimal(codes.next()?)?;
    let (code, keypad) = decode_code(primary);
    let mut event = KeyEvent::press(code).keypad(keypad);
    for alternate in codes.take(2) {
        if alternate.is_empty() {
            continue;
        }
        let (alt, _) = decode_code(decimal(alternate)?);
        event = event.alternate(alt);
    }
    if let Some(field) = parts.next() {
        let (modifiers, kind) = if field.is_empty() {
            (Modifiers::new(), KeyKind::Press)
        } else {
            parse_mods_kind(field)?
        };
        event = event.modifiers(modifiers).kind(kind);
    }
    if let Some(text) = parts.next().filter(|text| !text.is_empty()) {
        let decoded = decode_text(text)?;
        if !decoded.is_empty() {
            event = event.text(decoded);
        }
    }
    Some(event)
}

pub(crate) fn encode_key(event: &KeyEvent) -> Vec<u8> {
    let simple = event.kind == KeyKind::Press
        && event.modifiers == Modifiers::new()
        && event.text.is_none()
        && event.alternates.iter().all(Option::is_none)
        && !event.keypad;
    if simple && let Some(bytes) = encode_plain(event.code) {
        return bytes;
    }
    if let KeyCode::F(number @ 1..=12) = event.code
        && event.text.is_none()
        && event.alternates.iter().all(Option::is_none)
        && !event.keypad
    {
        return encode_f_tilde(number, event);
    }
    encode_kitty(event)
}

fn encode_plain(code: KeyCode) -> Option<Vec<u8>> {
    let bytes: &[u8] = match code {
        KeyCode::Char(ch) if !ch.is_control() => {
            let mut buf = [0; 4];
            return Some(ch.encode_utf8(&mut buf).as_bytes().to_vec());
        }
        KeyCode::Enter => b"\r",
        KeyCode::Esc => b"\x1b",
        KeyCode::Backspace => b"\x7f",
        KeyCode::Tab => b"\t",
        KeyCode::Up => b"\x1b[A",
        KeyCode::Down => b"\x1b[B",
        KeyCode::Right => b"\x1b[C",
        KeyCode::Left => b"\x1b[D",
        KeyCode::Home => b"\x1b[H",
        KeyCode::End => b"\x1b[F",
        KeyCode::Insert => b"\x1b[2~",
        KeyCode::Delete => b"\x1b[3~",
        KeyCode::PageUp => b"\x1b[5~",
        KeyCode::PageDown => b"\x1b[6~",
        KeyCode::F(1) => b"\x1b[11~",
        KeyCode::F(2) => b"\x1b[12~",
        KeyCode::F(3) => b"\x1b[13~",
        KeyCode::F(4) => b"\x1b[14~",
        KeyCode::F(5) => b"\x1b[15~",
        KeyCode::F(6) => b"\x1b[17~",
        KeyCode::F(7) => b"\x1b[18~",
        KeyCode::F(8) => b"\x1b[19~",
        KeyCode::F(9) => b"\x1b[20~",
        KeyCode::F(10) => b"\x1b[21~",
        KeyCode::F(11) => b"\x1b[23~",
        KeyCode::F(12) => b"\x1b[24~",
        _ => return None,
    };
    Some(bytes.to_vec())
}

fn encode_f_tilde(number: u8, event: &KeyEvent) -> Vec<u8> {
    let Some(code) = f_tilde_number(number) else {
        return encode_kitty(event);
    };
    if event.kind == KeyKind::Press && event.modifiers == Modifiers::new() {
        return format!("\u{1b}[{code}~").into_bytes();
    }
    format!(
        "\u{1b}[{code};{}{}~",
        event.modifiers.to_kitty(),
        kind_suffix(event.kind)
    )
    .into_bytes()
}

fn encode_kitty(event: &KeyEvent) -> Vec<u8> {
    let mut body = kitty_number(event.code, event.keypad).to_string();
    for alternate in event.alternates.into_iter().flatten() {
        body.push(':');
        body.push_str(&kitty_number(alternate, false).to_string());
    }
    body.push(';');
    body.push_str(&event.modifiers.to_kitty().to_string());
    if event.kind != KeyKind::Press {
        body.push_str(kind_suffix(event.kind));
    }
    if let Some(text) = &event.text {
        body.push(';');
        let mut first = true;
        for ch in text.chars() {
            if !first {
                body.push(':');
            }
            first = false;
            body.push_str(&u32::from(ch).to_string());
        }
    }
    let mut bytes = Vec::from(&b"\x1b["[..]);
    bytes.extend(body.into_bytes());
    bytes.push(b'u');
    bytes
}

fn kind_suffix(kind: KeyKind) -> &'static str {
    match kind {
        KeyKind::Press => "",
        KeyKind::Repeat => ":2",
        KeyKind::Release => ":3",
    }
}

fn f_tilde_number(number: u8) -> Option<u8> {
    Some(match number {
        1 => 11,
        2 => 12,
        3 => 13,
        4 => 14,
        5 => 15,
        6 => 17,
        7 => 18,
        8 => 19,
        9 => 20,
        10 => 21,
        11 => 23,
        12 => 24,
        _ => return None,
    })
}

fn kitty_number(code: KeyCode, keypad: bool) -> u32 {
    if keypad && let Some(number) = keypad_number(code) {
        return number;
    }
    match code {
        KeyCode::Char(ch) => u32::from(ch),
        KeyCode::Enter => 13,
        KeyCode::Esc => 27,
        KeyCode::Backspace => 127,
        KeyCode::Tab => 9,
        KeyCode::Left => 57_417,
        KeyCode::Right => 57_418,
        KeyCode::Up => 57_419,
        KeyCode::Down => 57_420,
        KeyCode::PageUp => 57_421,
        KeyCode::PageDown => 57_422,
        KeyCode::Home => 57_423,
        KeyCode::End => 57_424,
        KeyCode::Insert => 57_425,
        KeyCode::Delete => 57_426,
        KeyCode::F(number @ 13..=35) => 57_376 + u32::from(number - 13),
        KeyCode::F(number) => u32::from(number),
        KeyCode::CapsLock => 57_358,
        KeyCode::ScrollLock => 57_359,
        KeyCode::NumLock => 57_360,
        KeyCode::PrintScreen => 57_361,
        KeyCode::Pause => 57_362,
        KeyCode::Menu => 57_363,
        KeyCode::Other(number) => number,
    }
}

fn keypad_number(code: KeyCode) -> Option<u32> {
    Some(match code {
        KeyCode::Char('0') => 57_399,
        KeyCode::Char('1') => 57_400,
        KeyCode::Char('2') => 57_401,
        KeyCode::Char('3') => 57_402,
        KeyCode::Char('4') => 57_403,
        KeyCode::Char('5') => 57_404,
        KeyCode::Char('6') => 57_405,
        KeyCode::Char('7') => 57_406,
        KeyCode::Char('8') => 57_407,
        KeyCode::Char('9') => 57_408,
        KeyCode::Char('.') => 57_409,
        KeyCode::Char('/') => 57_410,
        KeyCode::Char('*') => 57_411,
        KeyCode::Char('-') => 57_412,
        KeyCode::Char('+') => 57_413,
        KeyCode::Enter => 57_414,
        KeyCode::Char('=') => 57_415,
        KeyCode::Char(',') => 57_416,
        KeyCode::Other(57_427) => 57_427,
        _ => return None,
    })
}

fn mods_from_body(body: &str) -> Option<(Modifiers, KeyKind)> {
    if body.is_empty() {
        return Some((Modifiers::new(), KeyKind::Press));
    }
    let mut parts = body.split(';');
    let _ignored = parts.next()?;
    match parts.next() {
        None => Some((Modifiers::new(), KeyKind::Press)),
        Some(field) => parse_mods_kind(field),
    }
}

fn parse_mods_kind(field: &str) -> Option<(Modifiers, KeyKind)> {
    if field.is_empty() {
        return Some((Modifiers::new(), KeyKind::Press));
    }
    let mut parts = field.split(':');
    let modifiers = Modifiers::from_kitty(decimal(parts.next()?)?);
    let kind = match parts.next() {
        None => KeyKind::Press,
        Some(value) => kind_from(decimal(value)?),
    };
    Some((modifiers, kind))
}

fn kind_from(value: u32) -> KeyKind {
    match value {
        2 => KeyKind::Repeat,
        3 => KeyKind::Release,
        _ => KeyKind::Press,
    }
}

fn tilde_code(number: u32) -> Option<KeyCode> {
    let narrow = |value: u32| u8::try_from(value).ok();
    match number {
        1 | 7 => Some(KeyCode::Home),
        2 => Some(KeyCode::Insert),
        3 => Some(KeyCode::Delete),
        4 | 8 => Some(KeyCode::End),
        5 => Some(KeyCode::PageUp),
        6 => Some(KeyCode::PageDown),
        11..=15 => Some(KeyCode::F(narrow(number - 10)?)),
        17..=21 => Some(KeyCode::F(narrow(number - 11)?)),
        23..=26 => Some(KeyCode::F(narrow(number - 12)?)),
        28..=29 => Some(KeyCode::F(narrow(number - 15)?)),
        31..=34 => Some(KeyCode::F(narrow(number - 17)?)),
        _ => None,
    }
}

fn decode_code(code: u32) -> (KeyCode, bool) {
    if let Some(parsed) = functional(code) {
        return parsed;
    }
    match code {
        27 => (KeyCode::Esc, false),
        13 => (KeyCode::Enter, false),
        9 => (KeyCode::Tab, false),
        8 | 127 => (KeyCode::Backspace, false),
        _ => match char::from_u32(code) {
            Some(ch) => (KeyCode::Char(ch), false),
            None => (KeyCode::Other(code), false),
        },
    }
}

fn functional(code: u32) -> Option<(KeyCode, bool)> {
    let key = match code {
        57_358 => KeyCode::CapsLock,
        57_359 => KeyCode::ScrollLock,
        57_360 => KeyCode::NumLock,
        57_361 => KeyCode::PrintScreen,
        57_362 => KeyCode::Pause,
        57_363 => KeyCode::Menu,
        57_376..=57_398 => {
            let number = 13 + (code - 57_376);
            KeyCode::F(u8::try_from(number).ok()?)
        }
        57_399 => KeyCode::Char('0'),
        57_400 => KeyCode::Char('1'),
        57_401 => KeyCode::Char('2'),
        57_402 => KeyCode::Char('3'),
        57_403 => KeyCode::Char('4'),
        57_404 => KeyCode::Char('5'),
        57_405 => KeyCode::Char('6'),
        57_406 => KeyCode::Char('7'),
        57_407 => KeyCode::Char('8'),
        57_408 => KeyCode::Char('9'),
        57_409 => KeyCode::Char('.'),
        57_410 => KeyCode::Char('/'),
        57_411 => KeyCode::Char('*'),
        57_412 => KeyCode::Char('-'),
        57_413 => KeyCode::Char('+'),
        57_414 => KeyCode::Enter,
        57_415 => KeyCode::Char('='),
        57_416 => KeyCode::Char(','),
        57_417 => KeyCode::Left,
        57_418 => KeyCode::Right,
        57_419 => KeyCode::Up,
        57_420 => KeyCode::Down,
        57_421 => KeyCode::PageUp,
        57_422 => KeyCode::PageDown,
        57_423 => KeyCode::Home,
        57_424 => KeyCode::End,
        57_425 => KeyCode::Insert,
        57_426 => KeyCode::Delete,
        57_427 => KeyCode::Other(57_427),
        _ => return None,
    };
    let keypad = (57_399..=57_416).contains(&code) || code == 57_427;
    Some((key, keypad))
}

fn decode_text(text: &str) -> Option<String> {
    let mut out = String::new();
    for part in text.split(':') {
        if part.is_empty() {
            continue;
        }
        let ch = char::from_u32(decimal(part)?)?;
        out.push(ch);
    }
    Some(out)
}
