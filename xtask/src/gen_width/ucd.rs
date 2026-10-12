// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::io;
use std::ops::RangeInclusive;

pub(crate) fn ranges_with(source: &str, value: &str) -> io::Result<Vec<RangeInclusive<u32>>> {
    let mut ranges = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let data = line.split('#').next().unwrap_or_default().trim();
        if data.is_empty() {
            continue;
        }
        let mut fields = data.split(';').map(str::trim);
        let (Some(points), Some(found)) = (fields.next(), fields.next()) else {
            return Err(malformed(index, line));
        };
        let range = code_points(points).ok_or_else(|| malformed(index, line))?;
        if found == value {
            ranges.push(range);
        }
    }
    Ok(ranges)
}

fn code_points(points: &str) -> Option<RangeInclusive<u32>> {
    let (start, end) = points.split_once("..").unwrap_or((points, points));
    Some(u32::from_str_radix(start, 16).ok()?..=u32::from_str_radix(end, 16).ok()?)
}

fn malformed(index: usize, line: &str) -> io::Error {
    io::Error::other(format!("line {}: malformed UCD data: {line}", index + 1))
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
