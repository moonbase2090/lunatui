# lunatui-test

Testing kit for lunatui: styled snapshots, virtual clock and vt100 oracle helpers.

PR 11 adds the vt100 oracle. `Oracle` feeds encoder bytes to the `vt100` parser. `diff_matches` checks that painting the previous grid and then the encoded diff reproduces the next grid, including text, the attributes the parser records, and color when the encoder is in truecolor mode. `fuzz_bytes` and `fuzz_campaign` drive that check from arbitrary grid writes. Styled-snapshot serialization still lives on `lunatui_core::Grid` until a later PR. The virtual clock is not in this crate yet.
