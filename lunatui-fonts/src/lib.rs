// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

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
    use super::{Family, Weight};

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
