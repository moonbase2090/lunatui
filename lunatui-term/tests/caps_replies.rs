// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_term::{
    Caps, ColorDepth, GraphicsProtocol, Hyperlinks, KeyboardProtocol, PrivateMode, ProbeEnv,
    ProbeReply, QUERIES, RecordedTerminal, WidthPolicy,
};

fn env_256() -> ProbeEnv {
    ProbeEnv {
        term: String::from("xterm-256color"),
        ..ProbeEnv::default()
    }
}

fn count_slice(haystack: &[u8], needle: &[u8]) -> usize {
    if needle.is_empty() || haystack.len() < needle.len() {
        return 0;
    }
    haystack
        .windows(needle.len())
        .filter(|window| *window == needle)
        .count()
}

#[test]
fn queries_batch_private_modes_and_end_with_device_attributes() {
    assert!(QUERIES.ends_with(b"\x1b[c"));
    for needle in [
        &b"\x1b[>0q"[..],
        &b"\x1b[?2026$p"[..],
        &b"\x1b[?2027$p"[..],
        &b"\x1b[?2004$p"[..],
        &b"\x1b[?1004$p"[..],
        &b"\x1b[?u"[..],
        &b"\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\"[..],
        &b"\x1b]10;?\x1b\\"[..],
        &b"\x1b]11;?\x1b\\"[..],
    ] {
        assert_eq!(count_slice(QUERIES, needle), 1);
    }
}

#[test]
fn graphics_reply_accepts_only_success_for_the_query_image() {
    let reply = ProbeReply::from_replies(&env_256(), b"\x1b_Gi=31;OK\x1b\\\x1b[?62c");
    assert_eq!(reply.caps.protocols.graphics, GraphicsProtocol::Kitty);
    for bytes in [
        &b"\x1b_Gi=31;EINVAL:bad data\x1b\\\x1b[?62c"[..],
        &b"\x1b_Gi=32;OK\x1b\\\x1b[?62c"[..],
        &b"\x1b_Gi=0;OK\x1b\\\x1b[?62c"[..],
        &b"\x1b_G;OK\x1b\\\x1b[?62c"[..],
        &b"\x1b_Ga=q,i=31,OK\x1b\\\x1b[?62c"[..],
        &b"\x1b_Gi=31;OK trailing text\x1b\\\x1b[?62c"[..],
    ] {
        let reply = ProbeReply::from_replies(&env_256(), bytes);
        assert_eq!(reply.caps.protocols.graphics, GraphicsProtocol::Unknown);
    }
}

#[test]
fn xterm_reply_leaves_mode_2027_unrecognized() {
    let reply = ProbeReply::from_replies(&env_256(), RecordedTerminal::Xterm.replies());
    assert!(reply.saw_device_attributes);
    assert_eq!(reply.caps.identity.program, "xterm");
    assert_eq!(reply.caps.identity.version, "384");
    assert_eq!(reply.caps.color, ColorDepth::Ansi256);
    assert_eq!(
        reply.caps.modes.synchronized_output,
        PrivateMode::Unrecognized
    );
    assert_eq!(
        reply.caps.modes.grapheme_clusters,
        PrivateMode::Unrecognized
    );
    assert_eq!(reply.caps.modes.bracketed_paste, PrivateMode::Reset);
    assert_eq!(reply.caps.modes.focus_events, PrivateMode::Reset);
    assert_eq!(reply.caps.protocols.keyboard, KeyboardProtocol::Unknown);
    assert_eq!(reply.caps.protocols.graphics, GraphicsProtocol::Unknown);
    assert_eq!(reply.caps.protocols.hyperlinks, Hyperlinks::Osc8);
    assert_eq!(reply.caps.width, WidthPolicy::Unicode);
    assert_eq!(reply.caps.grapheme_enable(), None);
    assert_eq!(reply.caps.background_dark, None);
}

#[test]
fn kitty_reply_enables_mode_2027_and_reports_graphics() {
    let reply = ProbeReply::from_replies(&env_256(), RecordedTerminal::Kitty.replies());
    assert!(reply.saw_device_attributes);
    assert_eq!(reply.caps.identity.program, "kitty");
    assert_eq!(reply.caps.identity.version, "0.36.4");
    assert_eq!(reply.caps.color, ColorDepth::TrueColor);
    assert_eq!(reply.caps.modes.synchronized_output, PrivateMode::Reset);
    assert_eq!(reply.caps.modes.grapheme_clusters, PrivateMode::Reset);
    assert_eq!(reply.caps.modes.bracketed_paste, PrivateMode::Set);
    assert_eq!(reply.caps.modes.focus_events, PrivateMode::Reset);
    assert_eq!(reply.caps.protocols.keyboard, KeyboardProtocol::Kitty);
    assert_eq!(reply.caps.protocols.graphics, GraphicsProtocol::Kitty);
    assert_eq!(reply.caps.width, WidthPolicy::Unicode);
    assert_eq!(reply.caps.background_dark, Some(true));
    assert_eq!(
        reply.caps.grapheme_enable(),
        Some(b"\x1b[?2027h".as_slice())
    );
}

#[test]
fn wezterm_reply_keeps_mode_2027_set_and_a_light_background() {
    let reply = ProbeReply::from_replies(&env_256(), RecordedTerminal::WezTerm.replies());
    assert_eq!(reply.caps.identity.program, "WezTerm");
    assert_eq!(reply.caps.identity.version, "20240203-110809-5046fc22");
    assert_eq!(reply.caps.modes.synchronized_output, PrivateMode::Set);
    assert_eq!(reply.caps.modes.grapheme_clusters, PrivateMode::Set);
    assert_eq!(reply.caps.modes.focus_events, PrivateMode::Set);
    assert_eq!(reply.caps.color, ColorDepth::TrueColor);
    assert_eq!(reply.caps.background_dark, Some(false));
    assert_eq!(reply.caps.grapheme_enable(), None);
    assert_eq!(reply.caps.width, WidthPolicy::Unicode);
}

#[test]
fn ghostty_reply_reports_reset_modes_and_unrecognized_focus() {
    let reply = ProbeReply::from_replies(&env_256(), RecordedTerminal::Ghostty.replies());
    assert_eq!(reply.caps.identity.program, "ghostty");
    assert_eq!(reply.caps.identity.version, "1.1.3");
    assert_eq!(reply.caps.modes.synchronized_output, PrivateMode::Reset);
    assert_eq!(reply.caps.modes.grapheme_clusters, PrivateMode::Reset);
    assert_eq!(reply.caps.modes.focus_events, PrivateMode::Unrecognized);
    assert_eq!(reply.caps.protocols.keyboard, KeyboardProtocol::Kitty);
    assert_eq!(reply.caps.protocols.graphics, GraphicsProtocol::Kitty);
    assert_eq!(reply.caps.color, ColorDepth::Ansi256);
    assert_eq!(
        reply.caps.grapheme_enable(),
        Some(b"\x1b[?2027h".as_slice())
    );
}

#[test]
fn iterm2_reply_falls_back_to_unicode_when_mode_2027_is_missing() {
    let reply = ProbeReply::from_replies(&env_256(), RecordedTerminal::Iterm2.replies());
    assert_eq!(reply.caps.identity.program, "iTerm2");
    assert_eq!(reply.caps.identity.version, "3.5.4");
    assert_eq!(reply.caps.modes.synchronized_output, PrivateMode::Reset);
    assert_eq!(
        reply.caps.modes.grapheme_clusters,
        PrivateMode::Unrecognized
    );
    assert_eq!(reply.caps.width, WidthPolicy::Unicode);
    assert_eq!(reply.caps.width.ambiguous_columns(), 1);
    assert_eq!(reply.caps.color, ColorDepth::TrueColor);
    assert_eq!(reply.caps.background_dark, Some(true));
    assert_eq!(reply.caps.protocols.hyperlinks, Hyperlinks::Osc8);
    assert_eq!(reply.caps.grapheme_enable(), None);
}

#[test]
fn tmux_reply_marks_every_queried_mode_unrecognized() {
    let reply = ProbeReply::from_replies(&env_256(), RecordedTerminal::Tmux.replies());
    assert_eq!(reply.caps.identity.program, "tmux");
    assert_eq!(reply.caps.identity.version, "3.4");
    assert_eq!(
        reply.caps.modes.synchronized_output,
        PrivateMode::Unrecognized
    );
    assert_eq!(
        reply.caps.modes.grapheme_clusters,
        PrivateMode::Unrecognized
    );
    assert_eq!(reply.caps.modes.bracketed_paste, PrivateMode::Unrecognized);
    assert_eq!(reply.caps.modes.focus_events, PrivateMode::Unrecognized);
    assert_eq!(reply.caps.protocols.hyperlinks, Hyperlinks::Unknown);
    assert_eq!(reply.caps.width, WidthPolicy::Unicode);
    assert_eq!(reply.caps.color, ColorDepth::Ansi256);
}

#[test]
fn replies_after_device_attributes_are_ignored() {
    let mut bytes = RecordedTerminal::Xterm.replies().to_vec();
    bytes.extend_from_slice(b"\x1b[?2027;2$y");
    let reply = ProbeReply::from_replies(&env_256(), &bytes);
    assert!(reply.saw_device_attributes);
    assert_eq!(
        reply.caps.modes.grapheme_clusters,
        PrivateMode::Unrecognized
    );
    assert_eq!(reply.caps.grapheme_enable(), None);
}

#[test]
fn apple_terminal_without_mode_2027_uses_ambiguous_wide() {
    let caps = Caps::from_env(&ProbeEnv {
        term_program: String::from("Apple_Terminal"),
        locale: String::from("en_US.UTF-8"),
        ..ProbeEnv::default()
    });
    assert_eq!(caps.width, WidthPolicy::AmbiguousWide);
    assert_eq!(caps.width.ambiguous_columns(), 2);
    assert_eq!(caps.protocols.hyperlinks, Hyperlinks::Osc8);
}

#[test]
fn mode_2027_reset_overrides_the_apple_terminal_fallback() {
    let env = ProbeEnv {
        term_program: String::from("Apple_Terminal"),
        locale: String::from("en_US.UTF-8"),
        ..ProbeEnv::default()
    };
    let reply = ProbeReply::from_replies(&env, b"\x1b[?2027;2$y\x1b[c");
    assert_eq!(reply.caps.modes.grapheme_clusters, PrivateMode::Reset);
    assert_eq!(reply.caps.width, WidthPolicy::Unicode);
    assert_eq!(
        reply.caps.grapheme_enable(),
        Some(b"\x1b[?2027h".as_slice())
    );
}

#[test]
fn locked_off_mode_2027_keeps_the_apple_terminal_fallback() {
    let env = ProbeEnv {
        term_program: String::from("Apple_Terminal"),
        ..ProbeEnv::default()
    };
    let reply = ProbeReply::from_replies(&env, b"\x1b[?2027;4$y\x1b[c");
    assert_eq!(reply.caps.modes.grapheme_clusters, PrivateMode::LockedOff);
    assert_eq!(reply.caps.width, WidthPolicy::AmbiguousWide);
    assert_eq!(reply.caps.grapheme_enable(), None);
}

#[test]
fn unknown_terminal_in_a_cjk_locale_uses_ambiguous_wide() {
    for locale in ["ja_JP.UTF-8", "ko_KR.UTF-8", "zh-Hans.UTF-8", "zh_CN.UTF-8"] {
        let caps = Caps::from_env(&ProbeEnv {
            locale: String::from(locale),
            ..ProbeEnv::default()
        });
        assert_eq!(caps.width, WidthPolicy::AmbiguousWide, "{locale}");
    }
}

#[test]
fn kitty_stays_unicode_in_a_cjk_locale_when_mode_2027_is_absent() {
    let caps = Caps::from_env(&ProbeEnv {
        term_program: String::from("kitty"),
        locale: String::from("ja_JP.UTF-8"),
        ..ProbeEnv::default()
    });
    assert_eq!(caps.width, WidthPolicy::Unicode);
}

#[test]
fn no_color_stays_mono_when_a_truecolor_reply_arrives() {
    let env = ProbeEnv {
        term: String::from("xterm-256color"),
        colorterm: String::from("truecolor"),
        no_color: true,
        ..ProbeEnv::default()
    };
    let reply = ProbeReply::from_replies(&env, RecordedTerminal::Kitty.replies());
    assert_eq!(reply.caps.color, ColorDepth::Mono);
    assert_eq!(reply.caps.background_dark, Some(true));
}

#[test]
fn force_color_overrides_no_color() {
    let caps = Caps::from_env(&ProbeEnv {
        colorterm: String::from("truecolor"),
        no_color: true,
        force_color: true,
        ..ProbeEnv::default()
    });
    assert_eq!(caps.color, ColorDepth::TrueColor);
}

#[test]
fn control_bytes_in_xtversion_are_stripped_from_the_name() {
    let reply = ProbeReply::from_replies(&env_256(), b"\x1bP>|kit\nty(1.0)\x1b\\\x1b[c");
    assert_eq!(reply.caps.identity.program, "kitty");
    assert_eq!(reply.caps.identity.version, "1.0");
    assert!(reply.caps.identity.program.chars().all(|c| !c.is_control()));
}

#[test]
fn unknown_bytes_do_not_set_modes_and_a_later_report_still_does() {
    let junk = b"\x1b[?9999;1$y\xff\x1b[999$yTRAILING";
    let early = ProbeReply::from_replies(&env_256(), junk);
    assert!(!early.saw_device_attributes);
    assert_eq!(
        early.caps.modes.synchronized_output,
        PrivateMode::Unreported
    );
    let mut bytes = junk.to_vec();
    bytes.extend_from_slice(b"\x1b[?2026;1$y\x1b[c");
    let reply = ProbeReply::from_replies(&env_256(), &bytes);
    assert!(reply.saw_device_attributes);
    assert_eq!(reply.caps.modes.synchronized_output, PrivateMode::Set);
}

#[test]
fn prismattyc_is_native_and_uses_unicode_widths() {
    let caps = Caps::from_env(&ProbeEnv {
        term_program: String::from("prismattyc"),
        ..ProbeEnv::default()
    });
    assert!(caps.identity.prismattyc_native);
    assert_eq!(caps.width, WidthPolicy::Unicode);
    assert_eq!(caps.protocols.hyperlinks, Hyperlinks::Osc8);
}

#[test]
fn graphics_detection_requires_positive_evidence_and_parses_physical_metrics()
-> Result<(), Box<dyn std::error::Error>> {
    let reply = ProbeReply::from_replies(
        &env_256(),
        b"\x1b[6;20;10t\x1b]11;rgb:1212/3434/5656\x1b\\\x1b[?62;4;22c",
    );
    assert_eq!(reply.caps.protocols.graphics, GraphicsProtocol::Sixel);
    let cell = reply.caps.cell_pixels.ok_or("physical metrics")?;
    assert_eq!((cell.width(), cell.height()), (10, 20));
    assert_eq!(reply.caps.background_rgb, Some([18, 52, 86]));
    for bytes in [
        &b"\x1b[6;0;10t\x1b[?62c"[..],
        &b"\x1b[6;20;0t\x1b[?62c"[..],
        &b"\x1b[6;20;10;1t\x1b[?62c"[..],
        &b"\x1b[6;65536;10t\x1b[?62c"[..],
    ] {
        assert!(
            ProbeReply::from_replies(&env_256(), bytes)
                .caps
                .cell_pixels
                .is_none()
        );
    }
    let reply = ProbeReply::from_replies(&env_256(), b"\x1b_Gi=31;OK\x1b\\\x1b[?62;4c");
    assert_eq!(reply.caps.protocols.graphics, GraphicsProtocol::Kitty);
    Ok(())
}

#[test]
fn inline_detection_requires_live_iterm_identity_and_both_ambiguous_f_tokens() {
    let reply = ProbeReply::from_replies(
        &env_256(),
        b"\x1bP>|iTerm2 3.6.0\x1b\\\x1b]1337;Capabilities=T3BFFSy\x1b\\\x1b[?62c",
    );
    assert_eq!(reply.caps.protocols.graphics, GraphicsProtocol::Iterm2);
    for bytes in [
        &b"\x1bP>|iTerm2 3.6.0\x1b\\\x1b]1337;Capabilities=BF\x1b\\\x1b[?62c"[..],
        &b"\x1b]1337;Capabilities=FF\x1b\\\x1b[?62c"[..],
        &b"\x1bP>|other 1\x1b\\\x1b]1337;Capabilities=FF\x1b\\\x1b[?62c"[..],
        &b"\x1bP>|iTerm2 3.6.0\x1b\\\x1b]1337;Capabilities=FfF\x1b\\\x1b[?62c"[..],
    ] {
        assert_eq!(
            ProbeReply::from_replies(&env_256(), bytes)
                .caps
                .protocols
                .graphics,
            GraphicsProtocol::Unknown
        );
    }
    let reply = ProbeReply::from_replies(&env_256(), b"\x1b]1337;Capabilities=Sx\x1b\\\x1b[?62c");
    assert_eq!(reply.caps.protocols.graphics, GraphicsProtocol::Sixel);
}

#[test]
fn scaled_cell_report_uses_physical_retina_pixels_over_the_point_report()
-> Result<(), Box<dyn std::error::Error>> {
    let reply = ProbeReply::from_replies(
        &env_256(),
        b"\x1b]1337;ReportCellSize=19.5;9.5;2\x1b\\\x1b[6;20;10t\x1b[?62c",
    );
    let cells = reply.caps.cell_pixels.ok_or("scaled cells")?;
    assert_eq!((cells.width(), cells.height()), (19, 39));
    for metrics in [
        "NaN;9;2",
        "20;inf;2",
        "20;9;0",
        "20;9;-1",
        "20;9;2;3",
        "999999;9;2",
    ] {
        let reply = ProbeReply::from_replies(
            &env_256(),
            format!("\x1b]1337;ReportCellSize={metrics}\x1b\\\x1b[?62c").as_bytes(),
        );
        assert_eq!(reply.caps.cell_pixels, None, "metrics {metrics}");
    }
    Ok(())
}

#[test]
fn probe_keeps_typed_keys_between_capability_replies() {
    let reply = ProbeReply::from_replies(
        &env_256(),
        b"q\x1b_Gi=31;OK\x1b\\\x1bOP\x1b[?0u\x1b[99;5u\x1b]11;rgb:0000/0000/0000\x1b\\\x1b[?62cZ",
    );
    assert_eq!(reply.caps.protocols.graphics, GraphicsProtocol::Kitty);
    assert_eq!(reply.pending_input, b"q\x1bOP\x1b[99;5uZ");
}

#[test]
fn graphics_metrics_and_background_may_arrive_after_device_attributes()
-> Result<(), Box<dyn std::error::Error>> {
    let reply = ProbeReply::from_replies(
        &env_256(),
        b"\x1b[?63;1;2;4c\x1b[6;33;17t\x1b]11;rgb:0000/0000/0000\x1b\\",
    );
    assert!(reply.saw_device_attributes);
    assert_eq!(reply.caps.protocols.graphics, GraphicsProtocol::Sixel);
    let pixels = reply.caps.cell_pixels.ok_or("late cell metrics")?;
    assert_eq!((pixels.width(), pixels.height()), (17, 33));
    assert_eq!(reply.caps.background_rgb, Some([0, 0, 0]));
    assert_eq!(reply.pending_input, []);
    Ok(())
}
