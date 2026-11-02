# Embedded terminal

The app owns a child PTY and its parsed screen separately. The widget borrows the screen and draws ordinary Lunatui cells. Native hosts consume those cells through the existing Grid backend; ANSI hosts encode the same cells.

```rust,ignore
let size = PtySize::new(columns, rows)?;
let mut child = PtyProcess::spawn(command, size)?;
let mut screen = TerminalScreen::new(size);
// Poll child.read into a bounded buffer from an app frame, then screen.feed(&buffer[..n]).
// Queue screen.take_replies back to the child; child.flush_input preserves partial writes.
let view = PseudoTerminal::new(&screen).state(WidgetState::Focused);
if let Some(bytes) = view.input(event) { child.queue_input(&bytes)?; }
view.render(area, cx);
```

`lunatui-term/pty` contains validated `PtySize` and, on Unix, `PtyProcess`. The helper owns its master descriptor, direct child, process group and bounded input queue. Standard input, output and error use a fresh controlling terminal. Arguments stay separate; the helper does not build shell command strings. Descriptor flags prevent inheritance across exec. Drop kills the running process group and reaps the direct child. A cached exit status prevents signals to a reaped PID. Detached descendants are outside that group. Windows process spawning is not implemented in this cut.

`lunatui-widgets/terminal` contains the screen and widget and is off by default, listed in `all-widgets`. Screen parsing uses the already pinned, approved `vt100` crate. It is a parser dependency, not a widget. Control strings pass through a bounded filter before reaching its dynamically sized OSC buffer. Titles stay local. Clipboard, graphics and child resize requests do not affect the host. Device status and cursor queries produce bounded replies for the app to send back.

Dimensions are 1–4096 columns/rows with at most 1,048,576 visible cells. History keeps 256 rows, control strings at most 4096 bytes, pending device replies at most 4096 bytes, and child input at most 65,536 bytes. A rejected input chunk changes none of the queue. A nonblocking write leaves its unwritten suffix queued. Read returning `WouldBlock` means the app should poll on a later frame. The example limits output processed per frame so a noisy child cannot create an unbounded drain loop.

The app can use its existing `needs_animation_frames` method to schedule polling. There is no runtime wake thread and no change to outer `Session`, event parsing or focus ownership. Resizing first updates the PTY, then the emulator. The runtime currently uses the dimensions in `RunOptions`; automatic outer-window resize discovery remains a runtime concern.

The view resolves default colors through theme tokens and keeps child-requested SGR colors. Focus enables keyboard/paste encoding and a local cursor; disabled state dims output and forwards no input. The child controls application-cursor and bracketed-paste modes. Common xterm navigation, function keys, control bytes and Alt combinations are supported. Mouse reporting, kitty key negotiation and graphics protocols are not implemented. An app that changes focus during a paste must finish or cancel that input transaction. The VT parser uses conventional character widths and bounded combining sequences rather than grapheme-width mode 2027.

The alternatives were extending the supplied scalar-only emulator or moving both parsing and processes into the widget. Reusing the approved parser avoids duplicating terminal control handling. Keeping process ownership separate makes recorded byte streams renderable on every platform, avoids background mutation of a widget and allows process tests to run in an isolated VM.

## Example and verification

Run `cargo run --release -p lunatui --features app,color-profiles --example embedded_terminal -- --workspace .` on Unix. Ctrl+] changes focus; q exits only while the editor is focused. The example requests a frame while the child is active and polls its nonblocking master in bounded batches. A child error stays visible and makes the scripted capture fail.

Run child-process checks in a clean Linux VM with `cargo test --locked -p lunatui-term --no-default-features --features pty --test pty -- --test-threads=1`. Pure screen tests use `cargo test --locked -p lunatui-widgets --no-default-features --features terminal,color-profiles --test pseudo_terminal`. Run `python3 scripts/check-terminal-mutants.py --process --bench` there to check behavioral mutations and a failed benchmark write. The script checks committed HEAD in an isolated archive and keeps logs under `target/terminal-mutants`.

`scripts/record-terminal.sh` creates an owned two-test fixture, prebuilds it, captures two real test runs and checks the terminal region. `CAPTURE_ONLY=1` needs no rasterizer in the VM. Copy its cast files unchanged, then use `RENDER_ONLY=1` with the pinned rasterizer on the workstation. `PROFILE=ansi16` and `THEME=light` capture the fallback and light theme. The deterministic `terminal` parity case participates in both widget benchmark entrypoints; it includes VT parsing and screen allocation on both sides, excluding process scheduling and terminal-emulator painting.
