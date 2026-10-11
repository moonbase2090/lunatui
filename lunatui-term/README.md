# lunatui-term

Terminal layer for lunatui: encoder, capability probing, session and input parser.

PR 09 adds `lunatui_term::Encoder`: diff-driven frame encoding with pen-state SGR, cursor moves and a crossterm-compatible reset trailer. PR 10 adds `lunatui_term::Session` on Unix: raw mode, alternate screen, synchronized-output wrapping, one `write_all` per frame, and restore on drop or panic. PR 12 adds `lunatui_term::Caps`: one batched probe (environment, DECRQM for modes 2026 and 2027, primary device attributes as the sentinel, 200 ms budget) and the width-policy fallback when mode 2027 is absent. PR 20 adds `lunatui_term::Parser`: legacy keys, kitty keyboard sequences (modifiers, press/repeat/release, alternate keys and associated text), SGR mouse including pixel mode, X10 mouse, streamed bracketed paste and focus. `Event::encode` round-trips those events. `fuzz_input_parser` rejects input that grows the hold buffer or a paste slice past its cap.
