// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::io;
use std::ops::RangeInclusive;

pub(crate) struct Sources<'a> {
    pub(crate) east_asian_width: &'a str,
    pub(crate) general_category: &'a str,
    pub(crate) core_properties: &'a str,
    pub(crate) emoji_data: &'a str,
    pub(crate) hangul_syllable_type: &'a str,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Tables {
    pub(crate) zero: Vec<RangeInclusive<u32>>,
    pub(crate) wide: Vec<RangeInclusive<u32>>,
}

pub(crate) fn classify(_sources: &Sources<'_>) -> io::Result<Tables> {
    Ok(Tables {
        zero: Vec::new(),
        wide: Vec::new(),
    })
}

pub(crate) fn render(_tables: &Tables, _version: (u8, u8, u8)) -> String {
    String::new()
}

#[cfg(test)]
mod tests {
    use std::ops::RangeInclusive;

    use super::{Sources, Tables, classify, render};

    const SOURCES: Sources<'static> = Sources {
        east_asian_width: "1100..115F ; W\n3000 ; F\n3164 ; W\nFF61..FF9F ; H\n",
        general_category: "0300..036F ; Mn\n20DD..20E0 ; Me\n302A..302D ; Mn\n0600..0605 ; Cf\n",
        core_properties: "00AD ; Default_Ignorable_Code_Point\n200B..200F ; Default_Ignorable_Code_Point\n3164 ; Default_Ignorable_Code_Point\n",
        emoji_data: "1F600..1F64F ; Emoji_Presentation\n231A..231B ; Emoji_Presentation\n00A9 ; Emoji\n",
        hangul_syllable_type: "1100..115F ; L\n1160..11A7 ; V\n11A8..11FF ; T\n",
    };

    #[test]
    fn classifies_zero_width_and_wide_code_points() -> std::io::Result<()> {
        let expected = Tables {
            zero: vec![
                0x0000..=0x001F,
                0x007F..=0x009F,
                0x0300..=0x036F,
                0x1160..=0x11FF,
                0x200B..=0x200F,
                0x20DD..=0x20E0,
                0x302A..=0x302D,
            ],
            wide: vec![
                0x1100..=0x115F,
                0x231A..=0x231B,
                0x3000..=0x3000,
                0x3164..=0x3164,
                0x1F600..=0x1F64F,
            ],
        };
        assert_eq!(classify(&SOURCES)?, expected);
        Ok(())
    }

    #[test]
    fn renders_the_tables_module() {
        let tables = Tables {
            zero: vec![0x0000..=0x001F, 0x0300..=0x036F],
            wide: vec![RangeInclusive::new(0x1100, 0x115F)],
        };
        let expected = "\
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub(crate) const UNICODE_VERSION: (u8, u8, u8) = (18, 0, 0);

#[rustfmt::skip]
pub(crate) static ZERO_WIDTH: [(u32, u32); 2] = [
    (0x0000, 0x001F), (0x0300, 0x036F),
];

#[rustfmt::skip]
pub(crate) static WIDE: [(u32, u32); 1] = [
    (0x1100, 0x115F),
];
";
        assert_eq!(render(&tables, (18, 0, 0)), expected);
    }

    #[test]
    fn renders_four_ranges_per_line() {
        let tables = Tables {
            zero: (0..5).map(|i| i * 16..=i * 16 + 1).collect(),
            wide: Vec::new(),
        };
        let rendered = render(&tables, (18, 0, 0));
        assert!(rendered.contains(
            "    (0x0000, 0x0001), (0x0010, 0x0011), (0x0020, 0x0021), (0x0030, 0x0031),\n    (0x0040, 0x0041),\n"
        ));
    }
}
