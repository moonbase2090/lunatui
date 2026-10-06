// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::io;
use std::ops::RangeInclusive;

pub(crate) fn ranges_with(_source: &str, _value: &str) -> io::Result<Vec<RangeInclusive<u32>>> {
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use std::ops::RangeInclusive;

    use super::ranges_with;

    const EAST_ASIAN_WIDTH: &str = "\
# EastAsianWidth-18.0.0.txt
0000..001F     ; N  # Cc    [32] <control-0000>..<control-001F>
1100..115F;W     # Lo    [96] HANGUL CHOSEONG KIYEOK..HANGUL CHOSEONG FILLER
231A..231B     ; W  # So     [2] WATCH..HOURGLASS

3000           ; F  # Zs         IDEOGRAPHIC SPACE
";

    #[test]
    fn collects_single_points_and_ranges_with_the_value() -> std::io::Result<()> {
        assert_eq!(
            ranges_with(EAST_ASIAN_WIDTH, "W")?,
            [0x1100..=0x115F, 0x231A..=0x231B]
        );
        assert_eq!(
            ranges_with(EAST_ASIAN_WIDTH, "F")?,
            [RangeInclusive::new(0x3000, 0x3000)]
        );
        Ok(())
    }

    #[test]
    fn matches_the_whole_value_not_a_prefix() -> std::io::Result<()> {
        let source =
            "0041 ; Default_Ignorable_Code_Point_Extra\n0042 ; Default_Ignorable_Code_Point\n";
        assert_eq!(
            ranges_with(source, "Default_Ignorable_Code_Point")?,
            [RangeInclusive::new(0x42, 0x42)]
        );
        Ok(())
    }

    #[test]
    fn rejects_a_malformed_code_point() {
        assert!(ranges_with("12G4 ; W\n", "W").is_err());
    }

    #[test]
    fn rejects_a_line_without_a_value() {
        assert!(ranges_with("1234\n", "W").is_err());
    }
}
