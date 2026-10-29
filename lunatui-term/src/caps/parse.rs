// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use super::{
    Caps, CellPixels, ColorDepth, GraphicsProtocol, KeyboardProtocol, PrivateMode, ProbeEnv,
    ProbeReply, refresh,
};

pub(crate) fn from_replies(env: &ProbeEnv, replies: &[u8]) -> ProbeReply {
    let mut caps = Caps::from_env(env);
    let mut saw_device_attributes = false;
    let mut saw_rgb = false;
    let mut background_dark = None;
    let mut inline_files = false;
    let mut sixel = false;
    let mut live_iterm = false;
    let mut scaled_cells = None;
    let mut consumed = vec![false; replies.len()];
    let mut scan = Scan::new(replies);
    while let Some(event) = scan.next_event() {
        if is_probe_response(&event) {
            consumed[scan.event_start..scan.index].fill(true);
        }
        match event {
            Event::Csi {
                params,
                intermediates,
                final_byte,
            } => {
                if final_byte == b'c' {
                    if intermediates.is_empty() && params.starts_with(b"?") {
                        sixel |= params[1..]
                            .split(|byte| *byte == b';')
                            .skip(1)
                            .any(|part| part == b"4");
                    }
                    saw_device_attributes = true;
                    continue;
                }
                if !saw_device_attributes || final_byte == b't' {
                    apply_csi(&mut caps, params, intermediates, final_byte);
                }
            }
            Event::Dcs(data) => {
                if !saw_device_attributes {
                    apply_xtversion(&mut caps, data);
                    live_iterm |= xtversion_text(data).is_some()
                        && caps.identity.program.eq_ignore_ascii_case("iterm2");
                }
            }
            Event::Osc(data) => {
                if let Some(metrics) = data.strip_prefix(b"1337;ReportCellSize=") {
                    scaled_cells = std::str::from_utf8(metrics).ok().and_then(scaled_cell_size);
                }
                if let Some(features) = data.strip_prefix(b"1337;Capabilities=") {
                    let (files, reported_sixel) = graphics_features(features);
                    inline_files |= files;
                    sixel |= reported_sixel;
                }
                if let Some(spec) = data.strip_prefix(b"11;") {
                    caps.background_rgb = std::str::from_utf8(spec).ok().and_then(color_rgb);
                }
                if !saw_device_attributes {
                    apply_osc(data, &mut saw_rgb, &mut background_dark);
                }
            }
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
    if let Some(cells) = scaled_cells {
        caps.cell_pixels = Some(cells);
    }
    if caps.protocols.graphics != GraphicsProtocol::Kitty {
        caps.protocols.graphics = if inline_files && live_iterm {
            GraphicsProtocol::Iterm2
        } else if sixel {
            GraphicsProtocol::Sixel
        } else {
            GraphicsProtocol::Unknown
        };
    }
    refresh(&mut caps, env);
    ProbeReply {
        caps,
        saw_device_attributes,
        pending_input: replies
            .iter()
            .zip(consumed)
            .filter_map(|(byte, consumed)| (!consumed).then_some(*byte))
            .collect(),
    }
}

#[cfg(unix)]
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
    if final_byte == b't' && intermediates.is_empty() {
        let mut parts = params.split(|byte| *byte == b';');
        if parts.next() == Some(&b"6"[..]) {
            let height = parts.next().and_then(parse_u16);
            let width = parts.next().and_then(parse_u16);
            if parts.next().is_none() {
                caps.cell_pixels = height.zip(width).and_then(|(h, w)| CellPixels::new(w, h));
            }
        }
    }
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

fn scaled_cell_size(metrics: &str) -> Option<CellPixels> {
    let mut parts = metrics.split(';');
    let height = parts.next()?.parse::<f64>().ok()?;
    let width = parts.next()?.parse::<f64>().ok()?;
    let scale = parts
        .next()
        .map_or(Some(1.0), |part| part.parse::<f64>().ok())?;
    if parts.next().is_some() || !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let dimension = |points: f64| {
        let pixels = (points * scale).round();
        if !pixels.is_finite() || pixels < 1.0 || pixels > f64::from(u16::MAX) {
            return None;
        }
        format!("{pixels:.0}").parse::<u16>().ok()
    };
    CellPixels::new(dimension(width)?, dimension(height)?)
}

fn graphics_features(mut features: &[u8]) -> (bool, bool) {
    let mut files = 0;
    let mut sixel = false;
    while features.first().is_some_and(u8::is_ascii_uppercase) {
        let end = features
            .iter()
            .skip(1)
            .position(|byte| !byte.is_ascii_lowercase())
            .map_or(features.len(), |index| index + 1);
        let token = &features[..end];
        features = &features[end..];
        let digits = features
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        if digits == 0 {
            files += u8::from(token == b"F");
            sixel |= token == b"Sx";
        }
        features = &features[digits..];
    }
    (files >= 2, sixel)
}

fn color_rgb(spec: &str) -> Option<[u8; 3]> {
    let spec = spec.trim();
    if let Some(hex) = spec.strip_prefix('#') {
        if hex.len() != 6 || !hex.is_ascii() {
            return None;
        }
        return Some([
            u8::from_str_radix(&hex[..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..], 16).ok()?,
        ]);
    }
    let lower = spec.to_ascii_lowercase();
    let mut parts = lower.strip_prefix("rgb:")?.split('/');
    let mut rgb = [0; 3];
    for channel in &mut rgb {
        let (value, max) = hex_channel(parts.next()?)?;
        *channel = u8::try_from(value * 255 / max).ok()?;
    }
    parts.next().is_none().then_some(rgb)
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
    let Some((control, status)) = text.strip_prefix('G').and_then(|body| body.split_once(';'))
    else {
        return false;
    };
    status == "OK" && control.split(',').any(|field| field == "i=31")
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

fn is_probe_response(event: &Event<'_>) -> bool {
    match event {
        Event::Csi {
            params,
            intermediates,
            final_byte,
        } => {
            *final_byte == b'c'
                || *final_byte == b't'
                || (*final_byte == b'u' && params.starts_with(b"?"))
                || (*final_byte == b'y' && *intermediates == b"$")
        }
        Event::Dcs(data) => xtversion_text(data).is_some(),
        Event::Osc(data) => [
            b"10;".as_slice(),
            b"11;",
            b"1337;Capabilities=",
            b"1337;ReportCellSize=",
        ]
        .iter()
        .any(|prefix| data.starts_with(prefix)),
        Event::Apc(data) => data.starts_with(b"G"),
    }
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
    event_start: usize,
}

impl<'a> Scan<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            index: 0,
            event_start: 0,
        }
    }

    fn next_event(&mut self) -> Option<Event<'a>> {
        let bytes = self.bytes;
        while self.index < bytes.len() {
            if bytes[self.index] != 0x1b {
                self.index += 1;
                continue;
            }
            let intro = bytes.get(self.index + 1).copied()?;
            self.event_start = self.index;
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
