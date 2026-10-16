// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_test::styled_snapshot;
use lunatui_theme::{Profile, Theme, Token, Variant};
use lunatui_widgets::{RenderCx, StatusDot, StatusState, Widget};

fn token_for(state: StatusState) -> Token {
    if matches!(state, StatusState::Ok) {
        Token::Ok
    } else if matches!(state, StatusState::Fail) {
        Token::Fail
    } else {
        Token::TextPrimary
    }
}

fn snapshot_with_profile(
    state: StatusState,
    variant: Variant,
    profile: Profile,
    width: u16,
    height: u16,
    x: u16,
    y: u16,
) -> String {
    let theme = Theme::builtin(variant);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let dot = StatusDot::new(state);
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme).profile(profile);
    dot.render(Rect::new(x, y, 1, 1), &mut cx);
    styled_snapshot(&grid, &clusters)
}

#[test]
fn status_dot_renders_ansi16_for_every_state_and_size() {
    for state in [StatusState::Ok, StatusState::Fail, StatusState::Idle] {
        for variant in [Variant::Dark, Variant::Light] {
            let theme = Theme::builtin(variant);
            let expected = format!(
                "{:?}",
                Profile::Ansi16.downsample(theme.resolve(token_for(state)))
            );
            for (width, height, x, y) in [(1_u16, 1_u16, 0, 0), (80, 24, 79, 23)] {
                let snap =
                    snapshot_with_profile(state, variant, Profile::Ansi16, width, height, x, y);
                assert!(snap.contains('●'));
                assert!(
                    snap.contains(expected.as_str()),
                    "state {state:?} variant {variant:?} at {width}x{height}"
                );
                assert!(snap.contains("Indexed("), "expected indexed fg in snapshot");
            }
        }
    }
}
