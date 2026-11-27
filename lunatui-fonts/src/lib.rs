// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

/// The characters a compact file keeps, as inclusive ranges: Latin with Latin-1 and Latin
/// Extended-A, general punctuation, currency, letterlike symbols, arrows, mathematical operators,
/// technical symbols, box drawing, block elements, geometric shapes, check marks and the
/// replacement character.
///
/// A compact file has exactly the characters in these ranges that its full file has.
pub const COMPACT_RANGES: [(char, char); 12] = [
    ('\u{0020}', '\u{007E}'),
    ('\u{00A0}', '\u{017F}'),
    ('\u{2000}', '\u{206F}'),
    ('\u{20A0}', '\u{20CF}'),
    ('\u{2100}', '\u{214F}'),
    ('\u{2190}', '\u{21FF}'),
    ('\u{2200}', '\u{22FF}'),
    ('\u{2300}', '\u{23FF}'),
    ('\u{2500}', '\u{259F}'),
    ('\u{25A0}', '\u{25FF}'),
    ('\u{2713}', '\u{2718}'),
    ('\u{FFFD}', '\u{FFFD}'),
];

/// A font family bundled with this crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Family {
    /// Inter, the default UI font.
    Inter,
    /// IBM Plex Sans, an alternative UI font.
    IbmPlexSans,
    /// Noto Sans, an alternative UI font and the first fallback.
    NotoSans,
    /// JetBrains Mono, the monospaced font.
    JetBrainsMono,
    /// Noto Emoji, monochrome emoji and the last fallback.
    NotoEmoji,
}

/// Weight of a bundled font file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Weight {
    /// Regular weight.
    Regular,
    /// Bold weight.
    Bold,
}

impl Family {
    /// Every bundled family.
    pub const ALL: [Self; 5] = [
        Self::Inter,
        Self::IbmPlexSans,
        Self::NotoSans,
        Self::JetBrainsMono,
        Self::NotoEmoji,
    ];

    /// The family name as the font files give it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Inter => "Inter",
            Self::IbmPlexSans => "IBM Plex Sans",
            Self::NotoSans => "Noto Sans",
            Self::JetBrainsMono => "JetBrains Mono",
            Self::NotoEmoji => "Noto Emoji",
        }
    }

    /// The font file for `weight`.
    ///
    /// Noto Emoji has one file, which serves both weights.
    #[must_use]
    pub const fn data(self, weight: Weight) -> &'static [u8] {
        match (self, weight) {
            (Self::Inter, Weight::Regular) => include_bytes!("../fonts/Inter-Regular.ttf"),
            (Self::Inter, Weight::Bold) => include_bytes!("../fonts/Inter-Bold.ttf"),
            (Self::IbmPlexSans, Weight::Regular) => {
                include_bytes!("../fonts/IBMPlexSans-Regular.ttf")
            }
            (Self::IbmPlexSans, Weight::Bold) => include_bytes!("../fonts/IBMPlexSans-Bold.ttf"),
            (Self::NotoSans, Weight::Regular) => include_bytes!("../fonts/NotoSans-Regular.ttf"),
            (Self::NotoSans, Weight::Bold) => include_bytes!("../fonts/NotoSans-Bold.ttf"),
            (Self::JetBrainsMono, Weight::Regular) => {
                include_bytes!("../fonts/JetBrainsMono-Regular.ttf")
            }
            (Self::JetBrainsMono, Weight::Bold) => {
                include_bytes!("../fonts/JetBrainsMono-Bold.ttf")
            }
            (Self::NotoEmoji, _) => include_bytes!("../fonts/NotoEmoji-Variable.ttf"),
        }
    }

    /// A smaller copy of the font file for `weight` that keeps only the characters in
    /// [`COMPACT_RANGES`], or `None` when the family has none. Inter and JetBrains Mono have
    /// compact files.
    ///
    /// A compact file is a quarter to a tenth the size of the full file and is parsed several times
    /// faster, for programs that must draw their first text quickly on a tight CPU budget.
    #[must_use]
    pub const fn compact(self, weight: Weight) -> Option<&'static [u8]> {
        match (self, weight) {
            (Self::Inter, Weight::Regular) => {
                Some(include_bytes!("../fonts/compact/Inter-Regular.ttf"))
            }
            (Self::Inter, Weight::Bold) => Some(include_bytes!("../fonts/compact/Inter-Bold.ttf")),
            (Self::JetBrainsMono, Weight::Regular) => {
                Some(include_bytes!("../fonts/compact/JetBrainsMono-Regular.ttf"))
            }
            (Self::JetBrainsMono, Weight::Bold) => {
                Some(include_bytes!("../fonts/compact/JetBrainsMono-Bold.ttf"))
            }
            _ => None,
        }
    }

    /// The SIL Open Font License text for this family, with its copyright line.
    #[must_use]
    pub const fn license(self) -> &'static str {
        match self {
            Self::Inter => include_str!("../fonts/licenses/Inter-OFL.txt"),
            Self::IbmPlexSans => include_str!("../fonts/licenses/IBMPlexSans-OFL.txt"),
            Self::NotoSans => include_str!("../fonts/licenses/NotoSans-OFL.txt"),
            Self::JetBrainsMono => include_str!("../fonts/licenses/JetBrainsMono-OFL.txt"),
            Self::NotoEmoji => include_str!("../fonts/licenses/NotoEmoji-OFL.txt"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{COMPACT_RANGES, Family, Weight};

    const SFNT_TRUETYPE: [u8; 4] = [0, 1, 0, 0];

    #[test]
    fn every_file_is_a_truetype_font() {
        for family in Family::ALL {
            for weight in [Weight::Regular, Weight::Bold] {
                assert!(
                    family.data(weight).starts_with(&SFNT_TRUETYPE),
                    "{family:?} {weight:?}"
                );
            }
        }
    }

    #[test]
    fn only_inter_and_jetbrains_mono_have_compact_truetype_files() {
        for family in Family::ALL {
            for weight in [Weight::Regular, Weight::Bold] {
                let compact = family.compact(weight);
                let expected = matches!(family, Family::Inter | Family::JetBrainsMono);
                assert_eq!(compact.is_some(), expected, "{family:?} {weight:?}");
                if let Some(bytes) = compact {
                    assert!(bytes.starts_with(&SFNT_TRUETYPE), "{family:?} {weight:?}");
                    assert!(
                        bytes.len() * 4 < family.data(weight).len(),
                        "{family:?} {weight:?} is {} bytes",
                        bytes.len()
                    );
                    assert_ne!(Some(bytes), family.compact(other(weight)), "{family:?}");
                }
            }
        }
    }

    #[test]
    fn compact_ranges_are_sorted_and_disjoint() {
        for (first, last) in COMPACT_RANGES {
            assert!(first <= last, "{first:?}..={last:?}");
        }
        for pair in COMPACT_RANGES.windows(2) {
            assert!(pair[0].1 < pair[1].0, "{pair:?}");
        }
    }

    const fn other(weight: Weight) -> Weight {
        match weight {
            Weight::Regular => Weight::Bold,
            Weight::Bold => Weight::Regular,
        }
    }

    #[test]
    fn regular_and_bold_are_different_files_except_emoji() {
        for family in Family::ALL {
            let same = family.data(Weight::Regular) == family.data(Weight::Bold);
            assert_eq!(same, family == Family::NotoEmoji, "{family:?}");
        }
    }

    #[test]
    fn every_license_is_the_ofl_with_a_copyright_line() {
        for family in Family::ALL {
            let text = family.license();
            assert!(text.starts_with("Copyright"), "{family:?}");
            assert!(
                text.contains("SIL OPEN FONT LICENSE Version 1.1"),
                "{family:?}"
            );
        }
    }
}
