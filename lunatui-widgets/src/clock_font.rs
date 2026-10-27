// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

pub(super) fn five_by_seven(ch: char) -> (&'static [&'static str; 7], usize) {
    const TWO: [&str; 7] = [
        " ### ", "#   #", "    #", "   # ", "  #  ", " #   ", "#####",
    ];
    const THREE: [&str; 7] = [
        "#####", "   # ", "  #  ", "   # ", "    #", "#   #", " ### ",
    ];
    const FOUR: [&str; 7] = [
        "   # ", "  ## ", " # # ", "#  # ", "#####", "   # ", "   # ",
    ];
    const SEVEN: [&str; 7] = [
        "#####", "    #", "   # ", "  #  ", " #   ", " #   ", " #   ",
    ];
    const COLON: [&str; 7] = [" ", " ", "#", " ", "#", " ", " "];
    const BLANK: [&str; 7] = ["     "; 7];

    match ch {
        '2' => (&TWO, 5),
        '3' => (&THREE, 5),
        '4' => (&FOUR, 5),
        '7' => (&SEVEN, 5),
        ':' => (&COLON, 1),
        _ => (&BLANK, 5),
    }
}
