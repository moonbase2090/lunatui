# lunatui-term

Terminal layer for lunatui: encoder, capability probing, session and input parser.

PR 09 adds `lunatui_term::Encoder`: diff-driven frame encoding with pen-state SGR, cursor moves and a crossterm-compatible reset trailer. Capability probing, session and input parsing land in later roadmap PRs.
