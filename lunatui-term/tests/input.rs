// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("../README.md")]

use lunatui_term::{
    Event, Focus, KeyCode, KeyEvent, KeyKind, Modifiers, MouseButton, MouseEvent, MouseKind,
    MouseUnit, Parser, Paste, Scroll, fuzz_input_parser,
};

fn parse(bytes: &[u8]) -> Vec<Event> {
    let mut parser = Parser::new();
    let mut events = parser.push(bytes);
    events.extend(parser.flush());
    events
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::press(code))
}

fn modified(code: KeyCode, modifiers: Modifiers) -> Event {
    Event::Key(KeyEvent::press(code).modifiers(modifiers))
}

fn assert_rows(rows: &[(&str, &[u8], &[Event])]) {
    for (name, bytes, expected) in rows {
        assert_eq!(parse(bytes), *expected, "{name}");
    }
}

#[test]
fn captured_legacy_keys_decode() {
    let rows: &[(&str, &[u8], &[Event])] = &[
        ("ascii", b"a", &[key(KeyCode::Char('a'))]),
        ("cjk", "日".as_bytes(), &[key(KeyCode::Char('日'))]),
        ("cr", b"\r", &[key(KeyCode::Enter)]),
        ("lf", b"\n", &[key(KeyCode::Enter)]),
        ("tab", b"\t", &[key(KeyCode::Tab)]),
        ("del", b"\x7f", &[key(KeyCode::Backspace)]),
        ("bs", b"\x08", &[key(KeyCode::Backspace)]),
        (
            "ctrl-a",
            b"\x01",
            &[modified(KeyCode::Char('a'), Modifiers::new().control())],
        ),
        (
            "ctrl-space",
            b"\x00",
            &[modified(KeyCode::Char(' '), Modifiers::new().control())],
        ),
        ("esc", b"\x1b", &[key(KeyCode::Esc)]),
        ("up", b"\x1b[A", &[key(KeyCode::Up)]),
        ("down", b"\x1b[B", &[key(KeyCode::Down)]),
        ("right", b"\x1b[C", &[key(KeyCode::Right)]),
        ("left", b"\x1b[D", &[key(KeyCode::Left)]),
        ("ss3-up", b"\x1bOA", &[key(KeyCode::Up)]),
        ("home", b"\x1b[H", &[key(KeyCode::Home)]),
        ("end", b"\x1b[F", &[key(KeyCode::End)]),
        ("home-tilde", b"\x1b[1~", &[key(KeyCode::Home)]),
        ("insert", b"\x1b[2~", &[key(KeyCode::Insert)]),
        ("delete", b"\x1b[3~", &[key(KeyCode::Delete)]),
        ("page-up", b"\x1b[5~", &[key(KeyCode::PageUp)]),
        ("page-down", b"\x1b[6~", &[key(KeyCode::PageDown)]),
        ("f5", b"\x1b[15~", &[key(KeyCode::F(5))]),
        ("f6", b"\x1b[17~", &[key(KeyCode::F(6))]),
        ("f1-ss3", b"\x1bOP", &[key(KeyCode::F(1))]),
        (
            "shift-up",
            b"\x1b[1;2A",
            &[modified(KeyCode::Up, Modifiers::new().shift())],
        ),
        (
            "ctrl-up",
            b"\x1b[1;5A",
            &[modified(KeyCode::Up, Modifiers::new().control())],
        ),
        (
            "backtab",
            b"\x1b[Z",
            &[modified(KeyCode::Tab, Modifiers::new().shift())],
        ),
        (
            "alt-a",
            b"\x1ba",
            &[modified(KeyCode::Char('a'), Modifiers::new().alt())],
        ),
    ];
    assert_rows(rows);
}

#[test]
fn captured_kitty_sequences_decode() {
    let rows: &[(&str, &[u8], &[Event])] = &[
        ("kitty-a", b"\x1b[97u", &[key(KeyCode::Char('a'))]),
        (
            "kitty-ctrl-c",
            b"\x1b[99;5u",
            &[modified(KeyCode::Char('c'), Modifiers::new().control())],
        ),
        (
            "kitty-release",
            b"\x1b[97;1:3u",
            &[Event::Key(
                KeyEvent::press(KeyCode::Char('a')).kind(KeyKind::Release),
            )],
        ),
        (
            "kitty-repeat",
            b"\x1b[97;1:2u",
            &[Event::Key(
                KeyEvent::press(KeyCode::Char('a')).kind(KeyKind::Repeat),
            )],
        ),
        ("kitty-up", b"\x1b[57419u", &[key(KeyCode::Up)]),
        (
            "kitty-alt-key",
            b"\x1b[97:65;2u",
            &[Event::Key(
                KeyEvent::press(KeyCode::Char('a'))
                    .alternate(KeyCode::Char('A'))
                    .modifiers(Modifiers::new().shift()),
            )],
        ),
        (
            "kitty-text",
            b"\x1b[97;1;104:105u",
            &[Event::Key(KeyEvent::press(KeyCode::Char('a')).text("hi"))],
        ),
        (
            "keypad-zero",
            b"\x1b[57399u",
            &[Event::Key(KeyEvent::press(KeyCode::Char('0')).keypad(true))],
        ),
    ];
    assert_rows(rows);
}

#[test]
fn captured_mouse_paste_and_focus_decode() {
    let rows: &[(&str, &[u8], &[Event])] = &[
        (
            "sgr-down",
            b"\x1b[<0;1;1M",
            &[Event::Mouse(MouseEvent {
                kind: MouseKind::Down(MouseButton::Left),
                column: 0,
                row: 0,
                modifiers: Modifiers::new(),
                unit: MouseUnit::Cells,
            })],
        ),
        (
            "sgr-up",
            b"\x1b[<0;1;1m",
            &[Event::Mouse(MouseEvent {
                kind: MouseKind::Up(MouseButton::Left),
                column: 0,
                row: 0,
                modifiers: Modifiers::new(),
                unit: MouseUnit::Cells,
            })],
        ),
        (
            "sgr-drag",
            b"\x1b[<32;5;6M",
            &[Event::Mouse(MouseEvent {
                kind: MouseKind::Drag(MouseButton::Left),
                column: 4,
                row: 5,
                modifiers: Modifiers::new(),
                unit: MouseUnit::Cells,
            })],
        ),
        (
            "sgr-wheel",
            b"\x1b[<64;1;1M",
            &[Event::Mouse(MouseEvent {
                kind: MouseKind::Scroll(Scroll::Up),
                column: 0,
                row: 0,
                modifiers: Modifiers::new(),
                unit: MouseUnit::Cells,
            })],
        ),
        (
            "sgr-move",
            b"\x1b[<35;1;1M",
            &[Event::Mouse(MouseEvent {
                kind: MouseKind::Move,
                column: 0,
                row: 0,
                modifiers: Modifiers::new(),
                unit: MouseUnit::Cells,
            })],
        ),
        (
            "sgr-alt",
            b"\x1b[<8;1;1M",
            &[Event::Mouse(MouseEvent {
                kind: MouseKind::Down(MouseButton::Left),
                column: 0,
                row: 0,
                modifiers: Modifiers::new().alt(),
                unit: MouseUnit::Cells,
            })],
        ),
        (
            "x10",
            b"\x1b[M   ",
            &[Event::Mouse(MouseEvent {
                kind: MouseKind::Down(MouseButton::Left),
                column: 0,
                row: 0,
                modifiers: Modifiers::new(),
                unit: MouseUnit::Cells,
            })],
        ),
        ("focus-in", b"\x1b[I", &[Event::Focus(Focus::In)]),
        ("focus-out", b"\x1b[O", &[Event::Focus(Focus::Out)]),
        (
            "paste",
            b"\x1b[200~hello\x1b[201~",
            &[
                Event::Paste(Paste::Start),
                Event::Paste(Paste::Chunk(String::from("hello"))),
                Event::Paste(Paste::End),
            ],
        ),
    ];
    assert_rows(rows);
}

#[test]
fn capability_replies_are_not_keys() {
    assert_eq!(parse(b"\x1b[?64;1c"), Vec::new());
    assert_eq!(parse(b"\x1b]11;rgb:0000/0000/0000\x07"), Vec::new());
    assert_eq!(parse(b"\x1b[?15u"), Vec::new());
}

#[test]
fn parse_of_encode_matches_the_event() {
    let events = [
        key(KeyCode::Char('a')),
        key(KeyCode::Char('日')),
        key(KeyCode::Enter),
        key(KeyCode::Esc),
        key(KeyCode::Backspace),
        key(KeyCode::Tab),
        key(KeyCode::Up),
        key(KeyCode::F(5)),
        key(KeyCode::F(12)),
        key(KeyCode::F(13)),
        key(KeyCode::CapsLock),
        modified(KeyCode::Char('a'), Modifiers::new().control()),
        modified(KeyCode::Up, Modifiers::new().shift()),
        Event::Key(KeyEvent::press(KeyCode::Char('a')).kind(KeyKind::Release)),
        Event::Key(KeyEvent::press(KeyCode::Char('a')).kind(KeyKind::Repeat)),
        Event::Key(
            KeyEvent::press(KeyCode::Char('a'))
                .alternate(KeyCode::Char('A'))
                .modifiers(Modifiers::new().shift()),
        ),
        Event::Key(KeyEvent::press(KeyCode::Char('a')).text("hi")),
        Event::Key(KeyEvent::press(KeyCode::Char('0')).keypad(true)),
        Event::Mouse(MouseEvent {
            kind: MouseKind::Down(MouseButton::Left),
            column: 3,
            row: 4,
            modifiers: Modifiers::new().shift(),
            unit: MouseUnit::Cells,
        }),
        Event::Mouse(MouseEvent {
            kind: MouseKind::Up(MouseButton::Right),
            column: 0,
            row: 0,
            modifiers: Modifiers::new(),
            unit: MouseUnit::Cells,
        }),
        Event::Mouse(MouseEvent {
            kind: MouseKind::Drag(MouseButton::Middle),
            column: 1,
            row: 2,
            modifiers: Modifiers::new(),
            unit: MouseUnit::Cells,
        }),
        Event::Mouse(MouseEvent {
            kind: MouseKind::Scroll(Scroll::Down),
            column: 8,
            row: 1,
            modifiers: Modifiers::new(),
            unit: MouseUnit::Cells,
        }),
        Event::Mouse(MouseEvent {
            kind: MouseKind::Move,
            column: 2,
            row: 2,
            modifiers: Modifiers::new(),
            unit: MouseUnit::Pixels,
        }),
        Event::Focus(Focus::In),
        Event::Focus(Focus::Out),
    ];
    for event in events {
        let mut parser = Parser::new();
        if let Event::Mouse(MouseEvent {
            unit: MouseUnit::Pixels,
            ..
        }) = event
        {
            parser.mouse_pixels(true);
        }
        let mut got = parser.push(&event.encode());
        got.extend(parser.flush());
        assert_eq!(got, vec![event]);
    }
}

#[test]
fn a_paste_round_trips_as_start_chunk_and_end() {
    let events = [
        Event::Paste(Paste::Start),
        Event::Paste(Paste::Chunk(String::from("hello"))),
        Event::Paste(Paste::End),
    ];
    let mut bytes = Vec::new();
    for event in &events {
        bytes.extend(event.encode());
    }
    assert_eq!(parse(&bytes), events);
}

#[test]
fn paste_split_across_pushes_concatenates() {
    let mut parser = Parser::new();
    let mut events = parser.push(b"\x1b[200~hel");
    events.extend(parser.push(b"lo\x1b[201~"));
    events.extend(parser.flush());
    assert_eq!(pasted(&events), "hello");
    assert!(
        events
            .iter()
            .any(|event| event == &Event::Paste(Paste::Start))
    );
    assert!(
        events
            .iter()
            .any(|event| event == &Event::Paste(Paste::End))
    );
}

#[test]
fn a_long_paste_is_split_into_1024_byte_chunks() {
    let payload = vec![b'x'; 2500];
    let mut bytes = b"\x1b[200~".to_vec();
    bytes.extend_from_slice(&payload);
    bytes.extend_from_slice(b"\x1b[201~");
    let events = parse(&bytes);
    assert_eq!(events.first(), Some(&Event::Paste(Paste::Start)));
    assert_eq!(events.last(), Some(&Event::Paste(Paste::End)));
    assert!(events.len() >= 4);
    let mut text = String::new();
    let mut chunks = true;
    for event in &events[1..events.len() - 1] {
        if let Event::Paste(Paste::Chunk(chunk)) = event {
            assert!(chunk.len() <= 1024);
            text.push_str(chunk);
        } else {
            chunks = false;
        }
    }
    assert!(chunks);
    assert_eq!(text, "x".repeat(2500));
}

#[test]
fn one_byte_at_a_time_matches_the_buffer() {
    let bytes = b"a\x1b[A\x1b[I";
    let whole = parse(bytes);
    let mut parser = Parser::new();
    let mut stepped = Vec::new();
    for byte in bytes {
        stepped.extend(parser.push(&[*byte]));
    }
    stepped.extend(parser.flush());
    assert_eq!(stepped, whole);
    assert_eq!(
        whole,
        vec![
            key(KeyCode::Char('a')),
            key(KeyCode::Up),
            Event::Focus(Focus::In),
        ]
    );
}

#[test]
fn a_hostile_csi_stays_within_4096_pending_bytes() {
    let mut bytes = vec![0x1b, b'['];
    bytes.extend(std::iter::repeat_n(b'3', 20_000));
    let mut parser = Parser::new();
    let _events = parser.push(&bytes);
    assert!(parser.pending() <= 4096);
    assert!(parser.pending() < bytes.len());
    let _tail = parser.flush();
    assert_eq!(parser.pending(), 0);
    assert!(fuzz_input_parser(&bytes).is_ok());
}

#[test]
fn fuzz_campaign_of_mixed_bytes_stays_bounded() {
    let mut state = 0x1234_5678u64;
    let mut bytes = Vec::with_capacity(4096);
    for _ in 0..4096 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let Ok(byte) = u8::try_from((state >> 33) & 0xff) else {
            continue;
        };
        bytes.push(byte);
    }
    assert!(fuzz_input_parser(&bytes).is_ok());
}

#[test]
fn pixel_mode_labels_the_same_sgr_bytes() {
    let mut parser = Parser::new();
    parser.mouse_pixels(true);
    let mut events = parser.push(b"\x1b[<0;10;20M");
    events.extend(parser.flush());
    assert_eq!(
        events,
        vec![Event::Mouse(MouseEvent {
            kind: MouseKind::Down(MouseButton::Left),
            column: 9,
            row: 19,
            modifiers: Modifiers::new(),
            unit: MouseUnit::Pixels,
        })]
    );
}

fn pasted(events: &[Event]) -> String {
    let mut text = String::new();
    for event in events {
        if let Event::Paste(Paste::Chunk(chunk)) = event {
            text.push_str(chunk);
        }
    }
    text
}
