// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/// A terminal whose reply transcript the probe is checked against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub enum RecordedTerminal {
    /// xterm 384: modes 2026 and 2027 unrecognized, paste and focus reset.
    Xterm,
    /// kitty 0.36.4: modes 2026 and 2027 reset, kitty keyboard and graphics, dark background.
    Kitty,
    /// `WezTerm`: modes 2026 and 2027 already set, light background.
    WezTerm,
    /// ghostty 1.1.3: modes 2026 and 2027 reset, focus unrecognized.
    Ghostty,
    /// iTerm2 3.5.4: mode 2026 reset, mode 2027 unrecognized.
    Iterm2,
    /// tmux 3.4: every queried mode unrecognized.
    Tmux,
}

const XTERM: &[u8] = b"\x1bP>|xterm(384)\x1b\\\x1b[?2026;0$y\x1b[?2027;0$y\x1b[?2004;2$y\x1b[?1004;2$y\x1b[?64;1;2;6;9;15;18;21;22c";

const KITTY: &[u8] = b"\x1bP>|kitty(0.36.4)\x1b\\\x1b[?2026;2$y\x1b[?2027;2$y\x1b[?2004;1$y\x1b[?1004;2$y\x1b[?31u\x1b_Ga=q,i=1,OK\x1b\\\x1b]11;rgb:0000/0000/0000\x1b\\\x1b[?62c";

const WEZTERM: &[u8] = b"\x1bP>|WezTerm 20240203-110809-5046fc22\x1b\\\x1b[?2026;1$y\x1b[?2027;1$y\x1b[?2004;2$y\x1b[?1004;1$y\x1b[?1u\x1b]11;rgb:ffff/ffff/ffff\x1b\\\x1b[?65;c";

const GHOSTTY: &[u8] = b"\x1bP>|ghostty 1.1.3\x1b\\\x1b[?2026;2$y\x1b[?2027;2$y\x1b[?2004;2$y\x1b[?1004;0$y\x1b[?0u\x1b_Ga=q,OK\x1b\\\x1b[?62;52c";

const ITERM2: &[u8] = b"\x1bP>|iTerm2 3.5.4\x1b\\\x1b[?2026;2$y\x1b[?2027;0$y\x1b[?2004;2$y\x1b[?1004;2$y\x1b]11;rgb:0000/0000/0000\x1b\\\x1b[?64;1;2c";

const TMUX: &[u8] =
    b"\x1bP>|tmux 3.4\x1b\\\x1b[?2026;0$y\x1b[?2027;0$y\x1b[?2004;0$y\x1b[?1004;0$y\x1b[?1;2c";

impl RecordedTerminal {
    /// Reply bytes through the primary device-attributes sentinel.
    #[must_use]
    pub fn replies(self) -> &'static [u8] {
        match self {
            Self::Xterm => XTERM,
            Self::Kitty => KITTY,
            Self::WezTerm => WEZTERM,
            Self::Ghostty => GHOSTTY,
            Self::Iterm2 => ITERM2,
            Self::Tmux => TMUX,
        }
    }
}
