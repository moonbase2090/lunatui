// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

pub(super) fn five_by_seven(ch: char) -> (&'static [&'static str; 7], usize) {
    const ZERO: [&str; 7] = [
        " ### ", "#   #", "#   #", "#   #", "#   #", "#   #", " ### ",
    ];
    const ONE: [&str; 7] = [
        "  #  ", " ##  ", "  #  ", "  #  ", "  #  ", "  #  ", " ### ",
    ];
    const TWO: [&str; 7] = [
        " ### ", "#   #", "    #", "   # ", "  #  ", " #   ", "#####",
    ];
    const THREE: [&str; 7] = [
        "#####", "   # ", "  #  ", "   # ", "    #", "#   #", " ### ",
    ];
    const FOUR: [&str; 7] = [
        "   # ", "  ## ", " # # ", "#  # ", "#####", "   # ", "   # ",
    ];
    const FIVE: [&str; 7] = [
        "#####", "#    ", "#### ", "    #", "    #", "#   #", " ### ",
    ];
    const SIX: [&str; 7] = [
        "  ## ", " #   ", "#    ", "#### ", "#   #", "#   #", " ### ",
    ];
    const SEVEN: [&str; 7] = [
        "#####", "    #", "   # ", "  #  ", " #   ", " #   ", " #   ",
    ];
    const EIGHT: [&str; 7] = [
        " ### ", "#   #", "#   #", " ### ", "#   #", "#   #", " ### ",
    ];
    const NINE: [&str; 7] = [
        " ### ", "#   #", "#   #", " ####", "    #", "   # ", " ##  ",
    ];
    const COLON: [&str; 7] = [" ", " ", "#", " ", "#", " ", " "];
    const BLANK: [&str; 7] = ["     "; 7];

    match ch {
        '0' => (&ZERO, 5),
        '1' => (&ONE, 5),
        '2' => (&TWO, 5),
        '3' => (&THREE, 5),
        '4' => (&FOUR, 5),
        '5' => (&FIVE, 5),
        '6' => (&SIX, 5),
        '7' => (&SEVEN, 5),
        '8' => (&EIGHT, 5),
        '9' => (&NINE, 5),
        ':' => (&COLON, 1),
        _ => (&BLANK, 5),
    }
}
