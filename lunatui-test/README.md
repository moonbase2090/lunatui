# lunatui-test

Testing kit for lunatui: styled snapshots, virtual clock, and vt100 oracle helpers.

## Styled snapshots

`styled_snapshot` serializes a [`Grid`](https://docs.rs/lunatui-core/latest/lunatui_core/struct.Grid.html) as a text layer plus run-length fg/bg/attr layers (`@text`, `@fg`, `@bg`, `@attrs`). Widget and theme tests use this format for readable diffs.

## Virtual clock

`Clock` is an injectable monotonic clock for deterministic animation tests (PRD §2.10). `lunatui-runtime` re-exports it for app animation code.

## Oracle

`Oracle` feeds encoder bytes to the `vt100` parser. `diff_matches` checks that painting the previous grid and then the encoded diff reproduces the next grid. `fuzz_bytes` and `fuzz_campaign` drive that check from arbitrary grid writes.
