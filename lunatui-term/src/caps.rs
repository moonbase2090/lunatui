// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod env;
mod parse;
#[cfg(unix)]
mod probe;
mod replies;

#[cfg(unix)]
pub use probe::probe_fd;
pub use replies::RecordedTerminal;

use std::time::Duration;

/// Hard ceiling for one capability probe, including a terminal that never answers.
pub const PROBE_BUDGET: Duration = Duration::from_millis(200);

/// Startup queries in one write, ending with primary device attributes (`CSI c`).
pub const QUERIES: &[u8] = b"\x1b[>0q\x1b[?2026$p\x1b[?2027$p\x1b[?2004$p\x1b[?1004$p\x1b[?u\x1b_Ga=q\x1b\\\x1b]10;?\x1b\\\x1b]11;?\x1b\\\x1b[c";

/// `CSI ? 2027 h`, sent when grapheme-cluster mode is supported and currently reset.
pub const GRAPHEME_ENABLE: &[u8] = b"\x1b[?2027h";

const UNICODE_TERMINALS: &[&str] = &[
    "kitty",
    "wezterm",
    "ghostty",
    "iterm",
    "alacritty",
    "foot",
    "vscode",
    "contour",
    "rio",
    "xterm",
    "tmux",
    "screen",
    "prismattyc",
    "warp",
];

const AMBIGUOUS_WIDE_TERMINALS: &[&str] = &["apple_terminal", "mintty"];

const HYPERLINK_TERMINALS: &[&str] = &[
    "kitty",
    "wezterm",
    "ghostty",
    "iterm",
    "vscode",
    "apple_terminal",
    "alacritty",
    "foot",
    "contour",
    "rio",
    "prismattyc",
    "xterm",
];

/// How many colors the probe will use.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[must_use]
pub enum ColorDepth {
    /// `NO_COLOR` is set, or `TERM` is unset or `dumb`.
    #[default]
    Mono,
    /// The terminal names a 16-color `TERM`.
    Ansi16,
    /// `TERM` contains `256color`.
    Ansi256,
    /// `COLORTERM` is `truecolor` or `24bit`, or an OSC color reply arrived.
    TrueColor,
}

/// What a DECRQM reply said about one private mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[must_use]
pub enum PrivateMode {
    /// No DECRPM reply arrived before primary device attributes.
    #[default]
    Unreported,
    /// The terminal does not recognize the mode (`Pm = 0`).
    Unrecognized,
    /// The mode is set, or permanently set.
    Set,
    /// The mode is recognized and currently reset.
    Reset,
    /// The mode is permanently reset and cannot be turned on.
    LockedOff,
}

/// DEC private modes the probe asks about.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[must_use]
pub struct DecModes {
    /// Mode 2026, synchronized output.
    pub synchronized_output: PrivateMode,
    /// Mode 2027, grapheme clusters.
    pub grapheme_clusters: PrivateMode,
    /// Mode 2004, bracketed paste.
    pub bracketed_paste: PrivateMode,
    /// Mode 1004, focus events.
    pub focus_events: PrivateMode,
}

/// Keyboard protocol the probe recognized.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[must_use]
pub enum KeyboardProtocol {
    /// No kitty keyboard reply arrived.
    #[default]
    Unknown,
    /// The terminal answered `CSI ? flags u`.
    Kitty,
}

/// Graphics protocol the probe recognized.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[must_use]
pub enum GraphicsProtocol {
    /// No kitty graphics reply arrived.
    #[default]
    Unknown,
    /// The kitty graphics query returned `OK`.
    Kitty,
}

/// Whether the terminal is treated as OSC 8 capable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[must_use]
pub enum Hyperlinks {
    /// The program name is not in the known OSC 8 set.
    #[default]
    Unknown,
    /// The program name is known to honor OSC 8 hyperlinks.
    Osc8,
}

/// Protocols reported beside the DEC modes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[must_use]
pub struct Protocols {
    /// Kitty keyboard protocol.
    pub keyboard: KeyboardProtocol,
    /// Kitty graphics protocol.
    pub graphics: GraphicsProtocol,
    /// OSC 8 hyperlinks, from the terminal name.
    pub hyperlinks: Hyperlinks,
}

/// Columns for East Asian Width Ambiguous scalars when mode 2027 is off.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[must_use]
pub enum WidthPolicy {
    /// Ambiguous scalars take one column. This is also the policy when mode 2027 is on.
    #[default]
    Unicode,
    /// Ambiguous scalars take two columns.
    AmbiguousWide,
}

impl WidthPolicy {
    /// Columns for one East Asian Width Ambiguous scalar under this policy.
    #[must_use]
    pub fn ambiguous_columns(self) -> usize {
        match self {
            Self::Unicode => 1,
            Self::AmbiguousWide => 2,
        }
    }
}

/// Which multiplexer environment variables were set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[must_use]
pub enum Multiplexer {
    /// Neither `TMUX` nor `STY` was set.
    #[default]
    None,
    /// `TMUX` was set.
    Tmux,
    /// `STY` was set.
    Screen,
    /// Both `TMUX` and `STY` were set.
    Both,
}

/// Terminal name and version from the environment or `XTVERSION`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[must_use]
pub struct TermIdentity {
    /// `XTVERSION` name, or `TERM_PROGRAM` when no version reply arrived.
    pub program: String,
    /// Version text from `XTVERSION`.
    pub version: String,
    /// The program or `TERM_PROGRAM` names Prismattyc.
    pub prismattyc_native: bool,
    /// Multiplexer variables from the environment.
    pub multiplexer: Multiplexer,
}

/// Environment the probe reads before any query.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[must_use]
pub struct ProbeEnv {
    /// `TERM`.
    pub term: String,
    /// `COLORTERM`.
    pub colorterm: String,
    /// `TERM_PROGRAM`.
    pub term_program: String,
    /// `LC_ALL`, else `LC_CTYPE`, else `LANG`.
    pub locale: String,
    /// `NO_COLOR` is set and non-empty.
    pub no_color: bool,
    /// `CLICOLOR_FORCE` is `1`, `true`, or `yes`.
    pub force_color: bool,
    /// `TMUX` and `STY`.
    pub multiplexer: Multiplexer,
}

impl ProbeEnv {
    /// Reads the probe environment from the process.
    pub fn from_process() -> Self {
        let multiplexer = match (env::var_is_set("TMUX"), env::var_is_set("STY")) {
            (true, true) => Multiplexer::Both,
            (true, false) => Multiplexer::Tmux,
            (false, true) => Multiplexer::Screen,
            (false, false) => Multiplexer::None,
        };
        Self {
            term: env::var_string("TERM"),
            colorterm: env::var_string("COLORTERM"),
            term_program: env::var_string("TERM_PROGRAM"),
            locale: env::first_var(&["LC_ALL", "LC_CTYPE", "LANG"]),
            no_color: env::var_nonempty("NO_COLOR"),
            force_color: env::var_forces_color("CLICOLOR_FORCE"),
            multiplexer,
        }
    }
}

/// Capability result: color, modes, protocols, and the width fallback.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[must_use]
pub struct Caps {
    /// Color depth from the environment, upgraded when an OSC rgb reply arrives.
    pub color: ColorDepth,
    /// DEC private modes.
    pub modes: DecModes,
    /// Keyboard, graphics, and hyperlinks.
    pub protocols: Protocols,
    /// Width fallback. Mode 2027 set or reset forces [`WidthPolicy::Unicode`].
    pub width: WidthPolicy,
    /// Program, version, multiplexer, and Prismattyc.
    pub identity: TermIdentity,
    /// `Some(true)` when the OSC 11 background is dark.
    pub background_dark: Option<bool>,
}

impl Caps {
    /// Builds caps from the environment, with every queried mode still unreported.
    pub fn from_env(env: &ProbeEnv) -> Self {
        let mut caps = Self {
            color: color_from_env(env),
            identity: TermIdentity {
                program: env.term_program.clone(),
                multiplexer: env.multiplexer,
                ..TermIdentity::default()
            },
            ..Self::default()
        };
        refresh(&mut caps, env);
        caps
    }

    /// Bytes that turn grapheme-cluster mode on, when the terminal reported it reset.
    #[must_use = "the enable sequence is written to the terminal"]
    pub fn grapheme_enable(&self) -> Option<&'static [u8]> {
        if self.modes.grapheme_clusters == PrivateMode::Reset {
            Some(GRAPHEME_ENABLE)
        } else {
            None
        }
    }
}

/// Parsed replies plus whether primary device attributes arrived.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct ProbeReply {
    /// Caps after applying every reply that arrived before device attributes.
    pub caps: Caps,
    /// A primary device-attributes reply (`CSI` … `c`) was present.
    pub saw_device_attributes: bool,
}

impl ProbeReply {
    /// Environment caps, with no reply bytes.
    pub fn from_env(env: &ProbeEnv) -> Self {
        Self {
            caps: Caps::from_env(env),
            saw_device_attributes: false,
        }
    }

    /// Applies reply bytes that arrived before primary device attributes.
    pub fn from_replies(env: &ProbeEnv, replies: &[u8]) -> Self {
        parse::from_replies(env, replies)
    }
}

pub(crate) fn refresh(caps: &mut Caps, env: &ProbeEnv) {
    caps.identity.prismattyc_native =
        names_prismattyc(&caps.identity.program) || names_prismattyc(&env.term_program);
    caps.protocols.hyperlinks = if name_has(&caps.identity.program, HYPERLINK_TERMINALS) {
        Hyperlinks::Osc8
    } else {
        Hyperlinks::Unknown
    };
    caps.width = choose_width(
        &caps.identity.program,
        &env.locale,
        caps.modes.grapheme_clusters,
    );
}

fn choose_width(program: &str, locale: &str, graphemes: PrivateMode) -> WidthPolicy {
    if matches!(graphemes, PrivateMode::Set | PrivateMode::Reset) {
        return WidthPolicy::Unicode;
    }
    if name_has(program, AMBIGUOUS_WIDE_TERMINALS) {
        return WidthPolicy::AmbiguousWide;
    }
    if name_has(program, UNICODE_TERMINALS) || !locale_is_cjk(locale) {
        return WidthPolicy::Unicode;
    }
    WidthPolicy::AmbiguousWide
}

fn name_has(program: &str, keys: &[&str]) -> bool {
    let name = program.to_ascii_lowercase();
    keys.iter().any(|key| name.contains(key))
}

fn names_prismattyc(name: &str) -> bool {
    name.to_ascii_lowercase().contains("prismattyc")
}

fn locale_is_cjk(locale: &str) -> bool {
    let token = locale.split(['.', '@']).next().unwrap_or("");
    let lang = token.split(['_', '-']).next().unwrap_or("");
    matches!(lang.to_ascii_lowercase().as_str(), "ja" | "ko" | "zh")
}

fn color_from_env(env: &ProbeEnv) -> ColorDepth {
    if env.no_color && !env.force_color {
        return ColorDepth::Mono;
    }
    let colorterm = env.colorterm.to_ascii_lowercase();
    if colorterm == "truecolor" || colorterm == "24bit" {
        return ColorDepth::TrueColor;
    }
    let term = env.term.to_ascii_lowercase();
    if term.contains("256color") || term.contains("256colour") {
        return ColorDepth::Ansi256;
    }
    if term.is_empty() || term == "dumb" {
        if env.force_color {
            ColorDepth::Ansi16
        } else {
            ColorDepth::Mono
        }
    } else {
        ColorDepth::Ansi16
    }
}
