// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::io;
use std::ops::RangeInclusive;

use super::ucd::ranges_with;
use crate::check::LICENSE_HEADER;

const CODE_POINTS: usize = 0x11_0000;
const SOFT_HYPHEN: usize = 0xAD;
const CONTROLS: [RangeInclusive<u32>; 2] = [0x00..=0x1F, 0x7F..=0x9F];
const RANGES_PER_LINE: usize = 4;

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

pub(crate) fn classify(sources: &Sources<'_>) -> io::Result<Tables> {
    let mut widths = vec![1_u8; CODE_POINTS];
    paint(
        &mut widths,
        &ranges_with(sources.core_properties, "Default_Ignorable_Code_Point")?,
        0,
    );
    for value in ["W", "F"] {
        paint(
            &mut widths,
            &ranges_with(sources.east_asian_width, value)?,
            2,
        );
    }
    paint(
        &mut widths,
        &ranges_with(sources.emoji_data, "Emoji_Presentation")?,
        2,
    );
    for value in ["Mn", "Me"] {
        paint(
            &mut widths,
            &ranges_with(sources.general_category, value)?,
            0,
        );
    }
    for value in ["V", "T"] {
        paint(
            &mut widths,
            &ranges_with(sources.hangul_syllable_type, value)?,
            0,
        );
    }
    paint(&mut widths, &CONTROLS, 0);
    if let Some(soft_hyphen) = widths.get_mut(SOFT_HYPHEN) {
        *soft_hyphen = 1;
    }
    Ok(Tables {
        zero: runs(&widths, 0),
        wide: runs(&widths, 2),
    })
}

fn paint(widths: &mut [u8], ranges: &[RangeInclusive<u32>], width: u8) {
    for code_point in ranges.iter().flat_map(Clone::clone) {
        if let Some(slot) = usize::try_from(code_point)
            .ok()
            .and_then(|index| widths.get_mut(index))
        {
            *slot = width;
        }
    }
}

fn runs(widths: &[u8], width: u8) -> Vec<RangeInclusive<u32>> {
    let mut runs = Vec::new();
    let mut start = None;
    let mut last = 0;
    for (code_point, &found) in (0_u32..).zip(widths) {
        match (found == width, start) {
            (true, None) => start = Some(code_point),
            (false, Some(first)) => {
                runs.push(first..=code_point - 1);
                start = None;
            }
            _ => {}
        }
        last = code_point;
    }
    if let Some(first) = start {
        runs.push(first..=last);
    }
    runs
}

pub(crate) fn render(tables: &Tables, (major, minor, update): (u8, u8, u8)) -> String {
    format!(
        "{LICENSE_HEADER}\npub(crate) const UNICODE_VERSION: (u8, u8, u8) = ({major}, {minor}, {update});\n\n{}\n{}",
        static_ranges("ZERO_WIDTH", &tables.zero),
        static_ranges("WIDE", &tables.wide),
    )
}

fn static_ranges(name: &str, ranges: &[RangeInclusive<u32>]) -> String {
    let rows: Vec<String> = ranges
        .chunks(RANGES_PER_LINE)
        .map(|chunk| {
            let pairs: Vec<String> = chunk
                .iter()
                .map(|range| format!("(0x{:04X}, 0x{:04X})", range.start(), range.end()))
                .collect();
            format!("    {},", pairs.join(", "))
        })
        .collect();
    format!(
        "#[rustfmt::skip]\npub(crate) static {name}: [(u32, u32); {}] = [\n{}\n];\n",
        ranges.len(),
        rows.join("\n")
    )
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
// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

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
