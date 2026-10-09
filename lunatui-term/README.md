# lunatui-term

Terminal layer for lunatui: encoder, capability probing, session and input parser.

PR 09 adds `lunatui_term::Encoder`: diff-driven frame encoding with pen-state SGR, cursor moves and a crossterm-compatible reset trailer. PR 10 adds `lunatui_term::Session` on Unix: raw mode, alternate screen, synchronized-output wrapping, one `write_all` per frame, and restore on drop or panic. Capability probing and input parsing land in later roadmap PRs.
