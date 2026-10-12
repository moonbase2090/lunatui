// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use super::{
    Caps, ColorDepth, GraphicsProtocol, KeyboardProtocol, PrivateMode, ProbeEnv, ProbeReply,
    refresh,
};

pub(crate) fn from_replies(env: &ProbeEnv, replies: &[u8]) -> ProbeReply {
    let mut caps = Caps::from_env(env);
    let mut saw_device_attributes = false;
    let mut saw_rgb = false;
    let mut background_dark = None;
    let mut scan = Scan::new(replies);
    while let Some(event) = scan.next_event() {
        match event {
            Event::Csi {
                params,
                intermediates,
                final_byte,
            } => {
                if final_byte == b'c' {
                    saw_device_attributes = true;
                    break;
                }
                apply_csi(&mut caps, params, intermediates, final_byte);
            }
            Event::Dcs(data) => apply_xtversion(&mut caps, data),
            Event::Osc(data) => apply_osc(data, &mut saw_rgb, &mut background_dark),
            Event::Apc(data) => {
                if kitty_graphics_ok(data) {
                    caps.protocols.graphics = GraphicsProtocol::Kitty;
                }
            }
        }
    }
    if saw_rgb {
        if caps.color != ColorDepth::Mono {
            caps.color = ColorDepth::TrueColor;
        }
        caps.background_dark = background_dark;
    }
    refresh(&mut caps, env);
    ProbeReply {
        caps,
        saw_device_attributes,
    }
}

pub(crate) fn has_device_attributes(bytes: &[u8]) -> bool {
    let mut scan = Scan::new(bytes);
    while let Some(event) = scan.next_event() {
        if let Event::Csi {
            final_byte: b'c', ..
        } = event
        {
            return true;
        }
    }
    false
}

fn apply_csi(caps: &mut Caps, params: &[u8], intermediates: &[u8], final_byte: u8) {
    if final_byte == b'u' && intermediates.is_empty() && params.first() == Some(&b'?') {
        caps.protocols.keyboard = KeyboardProtocol::Kitty;
        return;
    }
    if final_byte == b'y' && intermediates == b"$" {
        apply_decrpm(caps, params);
    }
}

fn apply_decrpm(caps: &mut Caps, params: &[u8]) {
    let params = params.strip_prefix(b"?").unwrap_or(params);
    let Some(split) = params.iter().position(|byte| *byte == b';') else {
        return;
    };
    let Some(mode) = parse_u16(&params[..split]) else {
        return;
    };
    let Some(pm) = parse_u16(&params[split + 1..]) else {
        return;
    };
    let Some(state) = private_mode(pm) else {
        return;
    };
    let slot = match mode {
        2026 => &mut caps.modes.synchronized_output,
        2027 => &mut caps.modes.grapheme_clusters,
        2004 => &mut caps.modes.bracketed_paste,
        1004 => &mut caps.modes.focus_events,
        _ => return,
    };
    *slot = state;
}

fn private_mode(pm: u16) -> Option<PrivateMode> {
    match pm {
        0 => Some(PrivateMode::Unrecognized),
        1 | 3 => Some(PrivateMode::Set),
        2 => Some(PrivateMode::Reset),
        4 => Some(PrivateMode::LockedOff),
        _ => None,
    }
}

fn apply_xtversion(caps: &mut Caps, data: &[u8]) {
    let Some(text) = xtversion_text(data) else {
        return;
    };
    let (program, version) = split_version(text);
    if program.is_empty() {
        return;
    }
    caps.identity.program = program;
    caps.identity.version = version;
}

fn xtversion_text(data: &[u8]) -> Option<&str> {
    let index = data.windows(2).position(|window| window == b">|")?;
    let text = data.get(index + 2..)?;
    std::str::from_utf8(text).ok().map(str::trim)
}

fn split_version(text: &str) -> (String, String) {
    if let Some(open) = text.find('(')
        && let Some(relative_close) = text[open + 1..].find(')')
    {
        let name = clean(text[..open].trim());
        let version = clean(text[open + 1..open + 1 + relative_close].trim());
        if !name.is_empty() {
            return (name, version);
        }
    }
    if let Some((name, version)) = text.split_once(' ') {
        return (clean(name.trim()), clean(version.trim()));
    }
    (clean(text.trim()), String::new())
}

fn clean(text: &str) -> String {
    text.chars().filter(|c| !c.is_control()).take(64).collect()
}

fn apply_osc(data: &[u8], saw_rgb: &mut bool, background_dark: &mut Option<bool>) {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Some((code, spec)) = text.split_once(';') else {
        return;
    };
    if !spec.to_ascii_lowercase().contains("rgb:") && !spec.contains('#') {
        return;
    }
    *saw_rgb = true;
    if code.trim() == "11"
        && let Some(is_dark) = color_is_dark(spec)
    {
        *background_dark = Some(is_dark);
    }
}

fn color_is_dark(spec: &str) -> Option<bool> {
    let spec = spec.trim();
    if let Some(hex) = spec.strip_prefix('#') {
        return hex_color_is_dark(hex);
    }
    let lower = spec.to_ascii_lowercase();
    let rest = lower.strip_prefix("rgb:")?;
    let mut parts = rest.split('/');
    let (red, red_max) = hex_channel(parts.next()?)?;
    let (green, green_max) = hex_channel(parts.next()?)?;
    let (blue, blue_max) = hex_channel(parts.next()?)?;
    if red_max != green_max || green_max != blue_max {
        return None;
    }
    Some(channel_is_dark(red, green, blue, red_max))
}

fn hex_color_is_dark(hex: &str) -> Option<bool> {
    if hex.len() != 6 {
        return None;
    }
    let (red, _) = hex_channel(&hex[0..2])?;
    let (green, _) = hex_channel(&hex[2..4])?;
    let (blue, _) = hex_channel(&hex[4..6])?;
    Some(channel_is_dark(red, green, blue, 255))
}

fn hex_channel(text: &str) -> Option<(u32, u32)> {
    if text.is_empty() || text.len() > 4 {
        return None;
    }
    let mut value = 0u32;
    for byte in text.bytes() {
        let digit = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => return None,
        };
        value = value.checked_mul(16)?.checked_add(u32::from(digit))?;
    }
    let shift = text.len().checked_mul(4)?;
    let max = 1u32
        .checked_shl(u32::try_from(shift).ok()?)?
        .checked_sub(1)?;
    Some((value, max))
}

fn channel_is_dark(red: u32, green: u32, blue: u32, max: u32) -> bool {
    let luma = red
        .saturating_mul(30)
        .saturating_add(green.saturating_mul(59))
        .saturating_add(blue.saturating_mul(11));
    luma.saturating_mul(2) < max.saturating_mul(100)
}

fn kitty_graphics_ok(data: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(data) else {
        return false;
    };
    if !text.contains("a=q") {
        return false;
    }
    text.split([',', ';', ' ']).any(|part| part == "OK")
}

fn parse_u16(bytes: &[u8]) -> Option<u16> {
    if bytes.is_empty() || bytes.len() > 5 {
        return None;
    }
    let mut value = 0u16;
    for byte in bytes {
        let digit = byte.checked_sub(b'0')?;
        if digit > 9 {
            return None;
        }
        value = value.checked_mul(10)?.checked_add(u16::from(digit))?;
    }
    Some(value)
}

enum Event<'a> {
    Csi {
        params: &'a [u8],
        intermediates: &'a [u8],
        final_byte: u8,
    },
    Dcs(&'a [u8]),
    Osc(&'a [u8]),
    Apc(&'a [u8]),
}

struct Scan<'a> {
    bytes: &'a [u8],
    index: usize,
}

impl<'a> Scan<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, index: 0 }
    }

    fn next_event(&mut self) -> Option<Event<'a>> {
        let bytes = self.bytes;
        while self.index < bytes.len() {
            if bytes[self.index] != 0x1b {
                self.index += 1;
                continue;
            }
            let intro = bytes.get(self.index + 1).copied()?;
            let start = self.index + 2;
            match intro {
                b'[' => match take_csi(bytes, start) {
                    Take::Done(csi) => {
                        self.index = csi.end;
                        return Some(Event::Csi {
                            params: csi.params,
                            intermediates: csi.intermediates,
                            final_byte: csi.final_byte,
                        });
                    }
                    Take::Reject { at } => self.index = at,
                    Take::Incomplete => return None,
                },
                b'P' | b']' | b'_' => match take_terminated(bytes, start) {
                    TakePayload::Done { data, end } => {
                        self.index = end;
                        return Some(match intro {
                            b'P' => Event::Dcs(data),
                            b']' => Event::Osc(data),
                            _ => Event::Apc(data),
                        });
                    }
                    TakePayload::Reject { at } => self.index = at,
                    TakePayload::Incomplete => return None,
                },
                _ => self.index += 1,
            }
        }
        None
    }
}

struct Csi<'a> {
    params: &'a [u8],
    intermediates: &'a [u8],
    final_byte: u8,
    end: usize,
}

enum Take<'a> {
    Done(Csi<'a>),
    Reject { at: usize },
    Incomplete,
}

fn take_csi(bytes: &[u8], start: usize) -> Take<'_> {
    let mut index = start;
    let param_start = index;
    while let Some(byte) = bytes.get(index)
        && is_param(*byte)
    {
        index += 1;
        if index - param_start > 64 {
            return Take::Reject { at: index };
        }
    }
    let param_end = index;
    while let Some(byte) = bytes.get(index)
        && is_intermediate(*byte)
    {
        index += 1;
        if index - param_end > 8 {
            return Take::Reject { at: index };
        }
    }
    let Some(final_byte) = bytes.get(index).copied() else {
        return Take::Incomplete;
    };
    if final_byte == 0x1b {
        return Take::Reject { at: index };
    }
    if !(0x40..=0x7e).contains(&final_byte) {
        return Take::Reject { at: index };
    }
    Take::Done(Csi {
        params: &bytes[param_start..param_end],
        intermediates: &bytes[param_end..index],
        final_byte,
        end: index + 1,
    })
}

enum TakePayload<'a> {
    Done { data: &'a [u8], end: usize },
    Reject { at: usize },
    Incomplete,
}

fn take_terminated(bytes: &[u8], start: usize) -> TakePayload<'_> {
    let mut index = start;
    while let Some(byte) = bytes.get(index).copied() {
        if byte == 0x07 {
            return TakePayload::Done {
                data: &bytes[start..index],
                end: index + 1,
            };
        }
        if byte == 0x1b {
            if bytes.get(index + 1) == Some(&b'\\') {
                return TakePayload::Done {
                    data: &bytes[start..index],
                    end: index + 2,
                };
            }
            return TakePayload::Reject { at: index };
        }
        index += 1;
        if index - start > 256 {
            return TakePayload::Reject { at: index };
        }
    }
    TakePayload::Incomplete
}

fn is_param(byte: u8) -> bool {
    (0x30..=0x3f).contains(&byte)
}

fn is_intermediate(byte: u8) -> bool {
    (0x20..=0x2f).contains(&byte)
}
