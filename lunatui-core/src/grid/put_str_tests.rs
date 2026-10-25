// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use super::{ClusterTable, Grid};
use crate::{Attrs, Color, Style};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }

    fn small(&mut self, bound: u64) -> u16 {
        u16::try_from(self.below(bound)).unwrap_or(0)
    }
}

fn style(rng: &mut Rng) -> Style {
    let colors = [
        Color::Default,
        Color::Indexed(3),
        Color::Rgb(9, 8, 7),
        Color::Rgb(200, 100, 50),
    ];
    let pick = |index: u64| colors[usize::try_from(index % 4).unwrap_or(0)];
    let bits = rng.next();
    let attrs = Attrs::default()
        .set_bold(bits & 1 != 0)
        .set_dim(bits & 2 != 0);
    Style::new()
        .fg(pick(bits >> 8))
        .bg(pick(bits >> 16))
        .with_attrs(attrs)
}

#[test]
fn put_str_matches_the_grapheme_path() {
    const TEXTS: [&str; 12] = [
        "a",
        "hello world",
        "0123456789abcdefghij",
        "日",
        "x日y",
        "é",
        "tab\there",
        "line\nbreak",
        " ",
        "~}|{",
        "日本語です",
        "ab\u{301}c",
    ];
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for _ in 0..600 {
        let mut fast = Grid::new(14, 3);
        let mut fast_clusters = ClusterTable::new();
        let mut slow = fast.clone();
        let mut slow_clusters = ClusterTable::new();
        for _ in 0..14 {
            let text = TEXTS[usize::try_from(rng.below(12)).unwrap_or(0)];
            let (x, y, painted) = (rng.small(16), rng.small(4), style(&mut rng));
            fast.put_str(x, y, painted, text, &mut fast_clusters);
            slow.put_graphemes(x, y, painted, text, &mut slow_clusters);
            assert_eq!(fast, slow);
        }
    }
}
