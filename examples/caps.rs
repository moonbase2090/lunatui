// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("README.md")]

use std::fs::File;
use std::io::{self, IsTerminal, Write};

use lunatui_core::width;
use lunatui_term::{
    Caps, ColorDepth, GraphicsProtocol, Hyperlinks, KeyboardProtocol, Multiplexer, PrivateMode,
    ProbeEnv, WidthPolicy,
};

fn main() -> io::Result<()> {
    let env = ProbeEnv::from_process();
    let mut caps = Caps::from_env(&env);
    let probe = probe_note(&env, &mut caps);
    let mut stdout = io::stdout().lock();
    stdout.write_all(picture(&caps, &probe).as_bytes())?;
    stdout.flush()
}

fn probe_note(env: &ProbeEnv, caps: &mut Caps) -> String {
    if !io::stdout().is_terminal() {
        return String::from("not a tty");
    }
    probe_tty(env, caps)
}

#[cfg(unix)]
fn probe_tty(env: &ProbeEnv, caps: &mut Caps) -> String {
    use std::os::fd::AsFd;

    use lunatui_term::probe_fd;

    let Ok(tty) = File::options().read(true).write(true).open("/dev/tty") else {
        return String::from("no tty");
    };
    match probe_fd(tty.as_fd(), env) {
        Ok(reply) => {
            *caps = reply.caps;
            if reply.saw_device_attributes {
                String::from("da1")
            } else {
                String::from("timeout")
            }
        }
        Err(_) => String::from("error"),
    }
}

#[cfg(not(unix))]
fn probe_tty(env: &ProbeEnv, caps: &mut Caps) -> String {
    let _ = (env, caps);
    String::from("not a tty")
}

fn picture(caps: &Caps, probe: &str) -> String {
    let rows = [
        field("color", color_name(caps.color)),
        field("sync", mode_name(caps.modes.synchronized_output)),
        field("clusters", mode_name(caps.modes.grapheme_clusters)),
        field("paste", mode_name(caps.modes.bracketed_paste)),
        field("focus", mode_name(caps.modes.focus_events)),
        field("keyboard", keyboard_name(caps.protocols.keyboard)),
        field("graphics", graphics_name(caps.protocols.graphics)),
        field("links", hyperlink_name(caps.protocols.hyperlinks)),
        field("width", width_name(caps.width)),
        field("program", &shown(&caps.identity.program)),
        field("version", &shown(&caps.identity.version)),
        field("mux", mux_name(caps.identity.multiplexer)),
        field(
            "prismattyc",
            if caps.identity.prismattyc_native {
                "yes"
            } else {
                "no"
            },
        ),
        field("background", background_name(caps.background_dark)),
        field("probe", &shown(probe)),
    ];
    draw(&rows)
}

fn field(label: &str, value: &str) -> String {
    format!("{label:<11} {value}")
}

fn shown(value: &str) -> String {
    if value.contains('/') || value.contains('\\') {
        return String::from("redacted");
    }
    value.chars().filter(|c| !c.is_control()).take(64).collect()
}

fn color_name(color: ColorDepth) -> &'static str {
    match color {
        ColorDepth::Mono => "mono",
        ColorDepth::Ansi16 => "ansi16",
        ColorDepth::Ansi256 => "ansi256",
        ColorDepth::TrueColor => "truecolor",
    }
}

fn mode_name(mode: PrivateMode) -> &'static str {
    match mode {
        PrivateMode::Unreported => "unreported",
        PrivateMode::Unrecognized => "unrecognized",
        PrivateMode::Set => "set",
        PrivateMode::Reset => "reset",
        PrivateMode::LockedOff => "locked-off",
    }
}

fn keyboard_name(protocol: KeyboardProtocol) -> &'static str {
    match protocol {
        KeyboardProtocol::Unknown => "unknown",
        KeyboardProtocol::Kitty => "kitty",
    }
}

fn graphics_name(protocol: GraphicsProtocol) -> &'static str {
    match protocol {
        GraphicsProtocol::Unknown => "unknown",
        GraphicsProtocol::Kitty => "kitty",
    }
}

fn hyperlink_name(links: Hyperlinks) -> &'static str {
    match links {
        Hyperlinks::Unknown => "unknown",
        Hyperlinks::Osc8 => "osc8",
    }
}

fn width_name(policy: WidthPolicy) -> &'static str {
    match policy {
        WidthPolicy::Unicode => "unicode",
        WidthPolicy::AmbiguousWide => "ambiguous-wide",
    }
}

fn mux_name(multiplexer: Multiplexer) -> &'static str {
    match multiplexer {
        Multiplexer::None => "no",
        Multiplexer::Tmux => "tmux",
        Multiplexer::Screen => "screen",
        Multiplexer::Both => "tmux+screen",
    }
}

fn background_name(dark: Option<bool>) -> &'static str {
    match dark {
        None => "unknown",
        Some(true) => "dark",
        Some(false) => "light",
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
        let label = " Caps ";
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
    use super::{picture, shown};
    use lunatui_core::width;
    use lunatui_term::{Caps, ProbeEnv};

    #[test]
    fn apple_terminal_without_a_tty_shows_the_ambiguous_wide_fallback() {
        let caps = Caps::from_env(&ProbeEnv {
            term: String::from("xterm-256color"),
            colorterm: String::from("truecolor"),
            term_program: String::from("Apple_Terminal"),
            locale: String::from("en_US.UTF-8"),
            ..ProbeEnv::default()
        });
        assert_eq!(picture(&caps, "not a tty"), APPLE_TERMINAL_BOX);
    }

    #[test]
    fn paths_in_the_program_name_are_not_printed() {
        let caps = Caps::from_env(&ProbeEnv {
            term_program: String::from("/opt/terminal"),
            ..ProbeEnv::default()
        });
        let drawn = picture(&caps, "not a tty");
        assert!(!drawn.contains("/opt"));
        assert!(drawn.contains("redacted"));
        assert_eq!(shown("kit\nty"), "kitty");
        let mut expected = None;
        let mut aligned = true;
        let mut lines = 0usize;
        for line in drawn.lines() {
            lines += 1;
            match expected {
                None => expected = Some(width(line)),
                Some(value) if value == width(line) => {}
                Some(_) => aligned = false,
            }
        }
        assert!(lines > 1);
        assert!(aligned);
        assert!(expected.is_some());
    }

    const APPLE_TERMINAL_BOX: &str = "\
┌ Caps ──────────────────────┐
│ color       truecolor      │
│ sync        unreported     │
│ clusters    unreported     │
│ paste       unreported     │
│ focus       unreported     │
│ keyboard    unknown        │
│ graphics    unknown        │
│ links       osc8           │
│ width       ambiguous-wide │
│ program     Apple_Terminal │
│ version                    │
│ mux         no             │
│ prismattyc  no             │
│ background  unknown        │
│ probe       not a tty      │
└────────────────────────────┘
";
}
