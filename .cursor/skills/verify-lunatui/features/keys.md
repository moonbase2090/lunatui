# Keys example

The keys example decodes terminal input events and prints a summary box.

## Sub-features

- `keys-decode` runs the bundled capture when stdin is empty.

## How to get to it (user POV)

- Run `cargo run --example keys` from the repository root.

## Driving it with verify-lunatui

Preconditions:

- Doctor passed for this `RUN_ID`.

- **Decode capture.** `.cursor/skills/verify-lunatui/scripts/run-example.sh keys`. Exit code 0; log contains `keys` and decoded event labels (e.g. key/mouse/paste lines from the capture).

- **Proof.** Attach `artifacts/$RUN_ID/keys.log` or paste the rendered box from the log.

## Gotchas

- When stdin is non-empty, the example decodes piped bytes instead of the bundled capture; state that entry point explicitly in Proof if you use it.
