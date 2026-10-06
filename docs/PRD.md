# lunatui: Product Requirements Document (PRD)

*A hyperspeed Rust TUI library to replace ratatui. Owner: MB2090 (Moonbase 2090 LLC). Status: **draft for sign-off**. Builds on `/workspace/ratatui-breakdown.md` and the measurements in `/workspace/ratatui-perf/results.txt`. Canonical copies: `/workspace/tui-replacement-plan.md` and `/workspace/lunatui-prd.md`, kept identical.*

- **Name: `lunatui`** (recommended in §12; final confirmation is open decision D1). Every crate and snippet uses it, so a rename is one pass.
- **Binding sections:** §M (Development methodology) and §T (Team model and review protocol) are **binding working rules** for everyone contributing, human or AI. The PR acceptance template (§8.0) enforces them. Every other section is the product plan and may change through the decision process in §11.
- **License:** MPL-2.0. **Repo:** `github.com/moonbase2090/lunatui`. Open source, Rust.
- **Rules for numbers:** "measured" means taken from my ratatui v0.30.2 runs on a shared 8-vCPU Xeon box (indicative, not absolute). "Target" is a goal. "Estimate" is a judgment, not a measurement.

---

## 0. One-paragraph summary

Build a new TUI stack from scratch that keeps what ratatui got right and fixes its structural costs. What ratatui got right: the simple "render widgets into a rectangle" mental model, builder-style styling, and the diff-and-encode pipeline. What it costs: 48-byte cells, a full-grid diff every frame, a Cassowary solve on every layout cache miss, O(offset) wrapped scrolling, ~1,000 allocations per frame, non-atomic output, and no input, focus, animation, capability or theme layer.

The new core uses a **16-byte packed cell**, **row-level dirty tracking**, a **closed-form layout fast path**, a **wrap cache**, and a **cost-model output encoder** that writes **once per frame inside a synchronized update**. On top sit a runtime with a first-class input parser (kitty keyboard, SGR mouse, bracketed paste), **hit-testing and focus**, an **event-driven frame scheduler with animation**, and **theme tokens**.

Graphite becomes the flagship look on top of the core. Scorecard is the first real consumer, then SkillSaw. A Prismattyc-native backend gives real translucency and sub-cell rendering inside Prismattyc. A ratatui compatibility crate lets existing ratatui widgets and apps run on it. SIMD, and inline asm only where proven, comes as a late, benchmark-gated phase.

Work happens under binding rules (§M): tests first, benchmarks before optimizations, property, differential and fuzz testing, styled snapshots, design docs before hard-to-undo code, and early dogfooding in Scorecard. A fixed team protocol (§T) governs it: one implementer seat per PR, a blind Muse review that posts `SHIP <sha>`, and Claude Opus checkpoint reviews. The roadmap is **38 PRs to v0.1.0** (35 code PRs plus 3 design-doc PRs) and **45 PRs to Scorecard adoption**.

---

## M. Development methodology (binding)

These rules are **mandatory for every PR**. A PR that doesn't satisfy them doesn't get a `SHIP`, however good the code is. Exceptions need a written waiver from MB2090 in the PR thread.

### M1. Test-driven development: each PR starts with failing tests

- The **first commit of every PR adds tests that fail** against the base branch and states the behavior the PR will deliver. Implementation commits follow.
- **Evidence:** the PR body links the CI run of the red commit. CI job `tests-first` checks out the base branch with the PR's new and changed test files applied, and **requires at least one new test to fail**. Pure docs, CI and bench-only PRs are exempt, and must be labeled `no-tdd`.
- Bug fixes start with a regression test reproducing the bug.

### M2. Benchmark-driven development: benchmark and target first, CI fails slowdowns over 3%

- **Before any optimization,** the PR (or an earlier PR) adds the benchmark and writes the **numeric target** in the PR body. The target comes from §1.3 or a design doc. Optimization PRs without a pre-existing benchmark are rejected.
- **CI perf gate:** `gungraun` instruction counts on the gated bench set. **Any benchmark more than 3% slower than `main` fails CI.** Also hard-fail any increase in allocation counts on zero-alloc paths, or in encoder byte counts on the transcript scenes.
- The PR body must include a **ratatui comparison table** (§T2) for every scene or micro-bench the PR touches.

### M3. Property-based testing of invariants

Every core module ships `proptest` properties for its invariants. Required invariants (more are added as modules land):

| Module | Invariant |
|---|---|
| Diff + encoder | **Applying the diff/encoded bytes to the old frame equals the new frame**: `oracle(old_screen + encode(diff(old, new))) == new` for random old/new grids, including wide chars, clusters, continuation cells and style churn |
| Grid writes | Writes never split a wide character from its continuation cell; dirty spans cover every changed cell and nothing outside the written range |
| Layout | Closed-form result == solver result on non-conflicting inputs; segments sum to the area (minus spacing); no overlap unless `Overlap` was requested; monotonic in the area size |
| Wrap cache | Cached wrap == fresh wrap for any text, width and scroll; append-only updates == full re-wrap |
| Width | ASCII fast path == table lookup; SIMD == scalar for every dispatch level |
| Grapheme slots | intern → resolve round-trips; equal clusters get equal slot IDs within a frame and across consecutive frames (§2.1) |
| Input parser | `parse(encode(event)) == event` for all representable events; arbitrary bytes never panic and always make progress |
| Hit map | The topmost registered rect wins; lookups agree with a brute-force scan |

### M4. Differential testing against ratatui

- **Same scene, identical terminal end state.** For every scene in `benches/scenes/` (dashboard, log, table, gradient, plus each widget ported in Phase 4), render with pinned ratatui 0.30.x and with lunatui. Feed both byte streams into the vt100 oracle from the same starting screen, and **require identical final screens (text and styles)** after every frame of a scripted sequence (tick, scroll, resize, overlay).
- **Before widgets exist (Phases 1–2):** the same rule applies at grid level. Identical cell grids are encoded by ratatui's `CrosstermBackend` and by our encoder, and the oracle end states must match.
- **Intentional differences** (for example, a bug ratatui has that we don't) are listed in `tests/differential/ALLOWED.md`, each with a reason and a review by Muse.

### M5. Fuzzing of the input parser and the output encoder

- `cargo-fuzz` targets, required from the PR that introduces each component:
  - `fuzz_input_parser`: arbitrary bytes → no panic, bounded memory, progress guaranteed
  - `fuzz_encoder`: arbitrary grid operations → oracle equality (M3's diff invariant)
  - `fuzz_caps_replies`: arbitrary terminal replies → no panic, no misdetection crash
  - `fuzz_theme_loader`
- **Per PR:** 60 s per touched target in CI. **Nightly:** 1 h per target. Crashes become regression tests (M1) before the fix.

### M6. Snapshot tests with styles

- Every widget and every Graphite component has **styled snapshots**: a text layer plus run-length fg/bg/attr/underline/link layers, with token names when available (`lunatui-test` format, `insta`-managed). Each component is snapshotted in all of its states (focused, hovered, disabled, error, empty, overflow), at two sizes, and in truecolor plus 16-color profiles. Animations are snapshotted at fixed virtual-clock times.
- Snapshot changes require explicit acceptance in the PR (no blanket `cargo insta accept`). Muse reviews snapshot diffs as part of the API and visual check.

### M7. Design docs before code for hard-to-undo pieces

- **Three pieces need an approved design doc (ADR in `docs/adr/`) before any implementation PR:**
  - **DD-1: cell format** (before PR 06)
  - **DD-2: layout engine** (before PR 14)
  - **DD-3: public API** (`App`/`Ui`/`Widget`/`RenderCx`, IDs, memo; before PR 23)
- Each design doc covers: problem, options considered, chosen design, memory layout and invariants, benchmarks and targets, migration and compat impact, open questions, and the tests that will prove it.
- **Approval:** the Muse seat reviews it, then **Claude Opus does a checkpoint review** (§T4), then MB2090 signs off. Code PRs reference the approved ADR number.
- **Changes after approval** need an ADR amendment PR with the same review chain.

### M8. Dogfooding via Scorecard as early as possible

Scorecard consumes lunatui from the earliest point each capability exists. These are branches in the Scorecard repo, tracked in its issues:

| After lunatui PR | Scorecard dogfood step |
|---|---|
| PR 10 (session + encoder + sync output) | `scorecard --tui-preview` prints a static gate summary through `Grid` + encoder (inline, no input) |
| PR 19 (layout, text, virtualization) | Preview renders the real findings list from a run, virtualized, with layout |
| PR 23 (runtime `run()`) | `scorecard tui` dogfood branch: live, keyboard-navigable, run daily by MB2090 |
| PR 24–26 (hit/focus, animation, themes) | Mouse, focus ring, lightcycle placeholder, theme tokens |
| PR 40–42 | Graduates to Scorecard `main` and ships (adoption) |

- Every dogfood friction point becomes a lunatui issue labeled `dogfood`. API changes driven by dogfooding go through M7 if they touch DD-3.

---

## T. Team model and review protocol (binding)

### T1. Seats

| Seat | Who | Responsibility |
|---|---|---|
| **Implementer seat A** | An implementer agent or engineer | Builds **one PR at a time**, start to finish, following §M. Never opens a second PR before the current one has `SHIP` or is closed |
| **Implementer seat B** | Joins **around PR 15** | Second parallel track, also one PR at a time. Suggested split from PR 15: A takes layout and text (14–19 →), B takes input and runtime (20–22 →). They converge for PR 23 (public API) under DD-3 |
| **Muse seat** | Reviewer agent | **Blind review** of every PR. Posts `SHIP <sha>` or sends it back |
| **Claude Opus** | Checkpoint reviewer | Deep reviews at fixed checkpoints (§T4) |
| **MB2090** | Owner | Signs off design docs and milestones, decides §11 items, grants waivers |

### T2. What every PR body must contain (implementer)

1. Linked issue, roadmap PR number, and ADR reference if applicable (M7).
2. Link to the CI run of the **red tests-first commit** (M1).
3. Summary of tests added, by type: unit, property (which invariants), differential, fuzz targets, snapshots.
4. **Benchmark comparison against ratatui** for every touched scene or micro-bench:

   | Bench | ratatui 0.30.x | lunatui `main` | this PR | Δ vs main | target |
   |---|---|---|---|---|---|

   It shows wall-clock (criterion, local, with hardware stated) and instruction counts (gungraun). Columns that don't apply say "n/a", never left blank.
5. Allocation and byte counts where relevant.
6. Known limitations and follow-ups.

### T3. Muse blind review

- **Blind** means Muse reviews the diff, tests, CI results and benchmark artifacts **as produced by CI**, not the implementer's claims or reasoning. Muse re-runs, or reads the CI output for, the benchmarks itself. The PR body's numbers must match CI. A mismatch is grounds for send-back.
- **Muse posts `SHIP <sha>` (the exact head commit) only if all three hold:**
  1. **Tests:** every §M rule satisfied for this PR type; CI green including `tests-first`, property, differential, fuzz and snapshot jobs.
  2. **API shape:** consistent with the approved ADRs and existing conventions; no accidental public surface; docs on every public item.
  3. **Benchmarks:** targets met where stated; **any bench more than 3% slower than `main` → sent back**, even if CI's gate was somehow bypassed.
- Otherwise Muse posts `CHANGES <sha>` with a numbered list. Any new commit invalidates a previous `SHIP`, so Muse re-reviews the new head.
- Merge happens only on a `SHIP` matching the current head SHA (enforced by a merge-queue check).

### T4. Claude Opus checkpoint reviews

| Checkpoint | When | Scope |
|---|---|---|
| **CP-1: Cell format** | DD-1 before PR 06, and again after PR 08 (diff) | Memory layout, grapheme slot design, continuation cells, mode 2027 policy, diff/encoder invariants, benchmark evidence |
| **CP-2: Layout engine** | DD-2 before PR 14, and again after PR 16 | Closed-form algorithm correctness, solver fallback boundary, ratatui parity, performance |
| **CP-3: Public API** | DD-3 before PR 23, and again after PR 28 | Ergonomics, naming, extensibility, semver risk, async/threading story |
| **CP-4: Pre-v0.1** | Before tagging v0.1.0 (PR 38) | Whole-crate API audit, docs, perf report against §1.3, security and `unsafe` review, release checklist |

The Opus review output is posted as a comment and must be resolved (fixed or answered) before the checkpoint passes.

### T5. Commit authorship

- **All commits are authored and committed by mb2090 using the GitHub noreply address** (`322824348+mb2090@users.noreply.github.com`).
- **No other author identities and no `Co-authored-by` trailers.** Implementer and reviewer seats configure `user.name`/`user.email` accordingly.
- CI job `authorship` rejects any commit in the PR whose author or committer email differs.

---

## 1. Goals, non-goals, and measurable targets

### 1.1 Goals

1. **Fastest practical TUI pipeline in Rust.** Measured against ratatui on identical scenes, with numbers published and gated in CI.
2. **Frame-atomic, minimal output.** One `write()` per frame, synchronized updates when supported, and the fewest bytes of any Rust TUI library (to be proven by benchmarks).
3. **Complete application layer.** Input, focus, hit-testing, animation, scheduling, themes, and capability detection are built in. These are exactly the gaps listed in breakdown §5 and §7.
4. **Familiar API.** A ratatui user should be productive in an afternoon. Widgets still "render into a rect".
5. **Graceful degradation.** Truecolor → 256 → 16 colors, kitty keyboard → legacy, sync output → plain, with the same app code.
6. **Correctness on Unicode.** Grapheme-cluster cells with terminal-aware width policy, and fuzzed and oracle-tested output.
7. **A path for ratatui users.** A compatibility crate (§4).

### 1.2 Non-goals (for 1.0)

- Not a GUI toolkit. The cell grid stays the primary model. Pixels only arrive through optional protocols (kitty graphics, the Prismattyc native backend).
- No declarative macro DSL or reactive signals framework in core. Others can build those on top.
- No built-in async runtime dependency in core. Async is an optional adapter.
- No full-assembly core (§10).
- No embedded/MCU target as a 1.0 promise. Core stays `no_std + alloc` capable, but there's no hardware CI.
- No accessibility tree at 1.0. Role metadata gets recorded in the hit/focus map (cheap), and the AT-SPI or UIA export comes later.

### 1.3 Performance targets vs measured ratatui baselines

Scenes are the Graphite-like dashboard, log and table from `ratatui-perf` (ported 1:1 in PR-03).

| Metric | ratatui v0.30.2 (measured) | lunatui target | Stretch |
|---|---|---|---|
| Frame time, dashboard tick, **200×60** (end to end incl. encode) | 0.48 ms | **≤ 0.10 ms** | 0.05 ms |
| Frame time, dashboard tick, **400×120** | 1.75 ms | **≤ 0.30 ms** | 0.15 ms |
| Frame time, scroll every frame, 200×60 / 400×120 | 0.81 / 1.92 ms | ≤ 0.20 / ≤ 0.45 ms | — |
| Diff, unchanged 200×60 grid | 122 µs | **≤ 8 µs** (scalar), ≤ 4 µs (SIMD) | prototype measured 4.2 µs |
| Layout miss, 6 mixed constraints | 141 µs | **≤ 5 µs** | 1 µs |
| **Worst-case layout miss**, 20 × `Fill` | **11.3 ms** | **≤ 20 µs** (closed form) | — |
| Solver-fallback layouts (true conflicts), ≤ 20 constraints | 0.9–11 ms | ≤ 1 ms | — |
| Wrapped log, 5,000 lines, scroll 4,000, 400×120 | 10.4 ms | **≤ 0.5 ms** (wrap cache hit) | — |
| Allocations per frame, steady state, dashboard 200×60 | ~980 (220 KiB) | **0 in core**, ≤ 20 including the widget layer | 0 |
| Memory per cell / two 200×60 grids | 48 B / 1.1 MiB | 16 B / 375 KiB | — |
| `write()` calls per large frame (68 KB) | 67 (default `Stdout`) | **1** | 1 |
| Bytes, dashboard tick, 200×60 | 209 B | ≤ 160 B (estimate-based target) | — |
| Bytes, scroll frame, 200×60 | 9.6 KB | ≤ 7 KB (−27%) | ≤ 6 KB |
| Bytes, truecolor gradient full frame, 200×60 | 68 KB | ≤ 50 KB | — |
| Library-side input→write latency (event read → `write()` returns), p99 | not measured | **≤ 1 ms** at 200×60 | 0.3 ms |
| End-to-end input→photon | not measured | ≤ terminal's own latency + 1 refresh (measured per §6.3) | — |
| Idle CPU (no animation, no input) | app-dependent: ratatui has no scheduler, and the examples poll on a tick | **0 wakeups/s** from the library | — |
| CPU during a 60 fps animation, 200×60 | not measured | ≤ 1% of one core (estimate) | — |
| Capability probe at startup | n/a | ≤ 50 ms typical, 200 ms hard timeout (estimate) | — |

Targets are set from measured component costs plus the 16-byte-cell prototype (4.2 µs diff). Anything not yet prototyped is a target, not a promise. PR-02/03 establish the comparison harness before any optimization lands.

---

## 2. Core architecture

### 2.1 Packed cell (16 bytes)

```text
struct Cell {            // 16 bytes, Copy, repr(C), align 16 (one u128 compare)
  glyph: u32,            // 4-byte grapheme slot (see "Grapheme slot" below):
                         //   tag=0 -> inline Unicode scalar value (21 bits) for common single-codepoint graphemes
                         //   tag=1 -> ClusterId into the per-frame interned side table (multi-codepoint clusters)
                         //   special value CONTINUATION for the trailing cell(s) of a wide grapheme
  fg:    u32,            // 2-bit kind {Default, Indexed, Rgb, Token} + 24-bit payload
  bg:    u32,            // same
  meta:  u32,            // attrs:12 (bold dim italic underline-style:3 blink reverse hidden strike overline)
                         // width:2 (0=continuation, 1, 2) | flags:2 (skip, protected) | ext:16 (index into per-frame ExtTable)
}
```

- **Why 16 B:** a third of ratatui's 48 B. A whole row of 400 cells is 6.4 KB, which fits in L1. Equality is one `u128` compare, so a row compares like `memcmp`. The prototype measured 4.2 µs for a 200×60 diff (breakdown §8.2b).
- **Rare data goes in an `ExtTable`:** underline color, OSC 8 hyperlink ID, image placement. Its 16-bit index lives in `meta`, so the 99% case stays compact.
- **Grapheme slot (4 bytes), the binding design for DD-1:**
  - **Inline codepoint (tag bit 31 = 0):** any grapheme that is a single Unicode scalar value is stored directly (21 bits): ASCII, Latin, CJK, box drawing, block elements, braille, and single-codepoint emoji. That covers the vast majority of cells, with no lookup and no allocation.
  - **Tagged cluster ID (tag bit 31 = 1):** multi-codepoint clusters (ZWJ emoji, VS16/VS15 sequences, flags, combining marks, Hangul jamo sequences) are interned into a **per-frame side table** (`ClusterTable`: string → `ClusterId`, plus a width cache).
  - **ID stability across frames:** the diff compares the old frame (table N−1) with the new frame (table N). At frame start, table N is **seeded from N−1's live entries with the same IDs**, new clusters get fresh IDs, and entries unreferenced in frame N are dropped when N+1 is seeded. Equal clusters therefore have equal IDs in consecutive frames, so slot comparison stays a plain integer compare and the diff never touches strings. M3 property: equal clusters always yield equal IDs within a frame and across consecutive frames.
  - **Bounded:** the table is capped (for example 64k IDs). On overflow, the frame falls back to a full re-intern and full diff, never wrong output.
- **Wide characters use continuation cells.** A width-2 grapheme occupies its lead cell (width field = 2) and one **continuation cell** (glyph = `CONTINUATION`, width field = 0) that carries the same style. Grid write APIs maintain the pair atomically:
  - Overwriting either half clears the other half to a blank with the overwriter's style.
  - A wide grapheme that doesn't fit at the right edge is replaced by a blank.

  The encoder never emits continuation cells, and accounts for the cursor advancing by the lead's width (avoiding ratatui's #2651 class of bug).
- **Terminal width policy and mode 2027:** the capability probe (§2.7) queries **DECRQM `?2027`** (grapheme cluster mode). If supported, lunatui enables it so the terminal segments graphemes the same way we do. If not, the per-terminal width policy (§2.5) decides cluster widths, from captured width tables. Support for 2027 varies by terminal and is determined at runtime, never assumed.
- **Width is computed once at write time** and stored, so the diff never calls Unicode tables. In the ratatui experiment, width caching alone gained only 0–18%; it pays off here because the cell is also small and compared as a `u128`.
- **`Token` color kind** stores a theme-token ID, resolved at encode time, so theme switches don't require re-rendering (optional; see the open decision on cell format).
- **Alternative considered:** an 8-byte cell (glyph `u32` + interned style ID `u32`). It's even smaller, but every styled write needs a hash-and-intern step. Recommendation: 16 B (open decision D4).

### 2.2 Model: immediate-mode API, incremental core (recommended)

| Option | Upside | Downside |
|---|---|---|
| Pure immediate mode (ratatui) | Simplest mental model; state lives in the app | Full re-render and full diff every frame; no identity, so no hit/focus/animation memory |
| Fully retained tree (DOM-like) | Minimal work per change; natural for focus and animation | Large API surface, ownership and lifetime pain in Rust, harder to learn, two sources of truth |
| **Immediate API + incremental core** (recommended) | Apps write `fn view(&self, ui: &mut Ui)` each frame, as in ratatui. The core keeps grid damage, keyed memoization and an ID map behind the scenes | More internal machinery; memo keys must be correct (debug mode verifies them) |

How the incremental part works:
1. **Stable IDs:** `ui.id("findings")` or auto IDs from call-site plus key. They feed hit-testing, focus, animation state and memoization.
2. **Memoized subtrees:** `ui.memo(id, &state_hash, |ui| ...)`. If the hash and the rect are unchanged, the subtree's previous cells are kept (no re-render). They're copied only if the region moved, and grid damage isn't marked.
3. **No back-buffer reset.** The front buffer is retained, and writes mark row dirty bits plus a dirty column span. Regions not repainted this frame are cleared only if they were painted last frame and not this frame (tracked through the ID map).
4. **Debug verification mode** (`LUNATUI_VERIFY=1`) re-renders memoized subtrees and asserts they're identical, catching stale memo keys.

### 2.3 Dirty tracking

- Per row: a `dirty: bool` plus a `[min_x, max_x)` span, updated by every write API in O(1).
- The diff only visits dirty rows, and within them only the dirty span. Inside the span it compares `u128` cells and emits **runs** (contiguous changed cells), not single cells.
- **Full invalidation** (resize, theme change without token colors, explicit `invalidate()`) falls back to a full diff, which is still fast at about 4–8 µs for 200×60 (target).

### 2.4 Layout engine

- **API:** a `Layout`/`Constraint` family deliberately close to ratatui (`Length`, `Percentage`, `Ratio`, `Min`, `Max`, `Fill`, flex modes, spacing and overlap), plus `Grid` (2-D tracks), `Stack` (z-layers) and **intrinsic sizing** (`Auto`: ask the widget for its preferred size).
- **Closed-form fast path (O(n)):** handles combinations without conflicting bounds. Steps: allocate `Length`, then `Percentage`/`Ratio` (floor plus largest-remainder rounding), then distribute the remainder across `Fill` weights and `Min`/`Max` through a water-filling pass (clamp, then redistribute iteratively, at most n passes, so O(n²) worst case with tiny constants). Flex modes position the leftover space.
- **Solver fallback:** `kasuari` only when the fast path detects a genuinely over-constrained mix (for example `Max` < `Min` conflicts with priorities). Both paths are behind the same API. A property test checks they produce identical rects on random inputs, and checks agreement with ratatui's results where semantics match (PR-15).
- **Memo:** per-frame memo by a `(rect, layout_hash)` key with no `Vec` clone, plus a small LRU across frames. Animated rects are cheap anyway because the fast path costs microseconds.
- **Animation-friendly:** `Layout::lerp(a, b, t)` interpolates between two solved layouts without re-solving.

### 2.5 Text and wrapping cache

- **Borrowed-first text:** `Span<'a>` holds `&'a str` or `Arc<str>` plus a `Style`. Lines and texts are built in a frame arena (bump allocator reset each frame), so they cost **0 heap allocations** in steady state.
- **Measurement:** an ASCII fast path (SIMD later), with grapheme segmentation only for non-ASCII bytes. Width comes from our own generated tables (PR-04), with a **terminal width policy** (Unicode default, plus per-terminal overrides for emoji/VS16, and DEC mode 2027 grapheme-cluster mode when the terminal reports it; terminal support for 2027 varies and must be probed).
- **`WrapCache`:** keyed by `(content_id or hash, width, wrap options)`. It stores a line-break index (logical line → first visual row, prefix sums), so scrolling to visual row N is an O(log n) binary search instead of re-wrapping from the top. Appending lines (logs) wraps only the new tail. Offsets are `u32`/`usize`, never `u16`.
- **Virtualized data:** a `RowSource` trait (`len()`, `row(i, &mut RowBuilder)`) for lists and tables, so only visible rows are ever built.

### 2.6 Output encoder

The output pipeline per frame: dirty runs → encoder → one `Vec<u8>` (reused across frames) → one `write_all` → done.

- **SGR coalescing:** track the terminal's current pen state (fg, bg, attrs, underline style and color, link) and emit only the delta, as **one combined CSI** (`ESC[1;3;38;2;r;g;bm`). Change fg and bg independently (ratatui always resends both, see breakdown §8.2h). Choose reset-plus-set when it's shorter than the delta.
- **Cursor-move cost model:** for each jump, pick the cheapest of: absolute `CUP`, `CUF`/`CUB` (relative), `CR`+`LF`, or **printing through** up to k unchanged cells when that's shorter than a move and the pen state matches.
- **Blank runs:** `ECH` (erase characters) or `EL` (erase to end of line) for runs of default-background blanks. With a non-default background, only use them where background-color-erase is known to behave (capability-gated). `REP` (repeat) only where probed.
- **Synchronized update:** wrap each frame in `CSI ? 2026 h … CSI ? 2026 l` when DECRQM reports support.
- **One write:** a reusable buffer sized to the high-water mark. Cursor show/hide/move are folded into the same buffer and emitted only when changed (ratatui re-emits them every frame, see PR #2752 upstream).
- **Backpressure:** measure `write()` duration. If the terminal is slow (SSH, slow emulator), the scheduler merges damage and skips intermediate animation frames instead of queueing them.

### 2.7 Capability detection

One batched probe at startup, with a hard timeout:

1. **Environment:** `TERM`, `COLORTERM`, `TERM_PROGRAM`, `NO_COLOR`, `CLICOLOR_FORCE`, multiplexer hints (`TMUX`, `STY`).
2. **One write of queries, terminated with DA1 (`CSI c`) as a sentinel.** Every terminal answers DA1, so any query answered before the DA1 reply is "supported" and anything not answered is "unsupported". This avoids per-query timeouts. Queries include:
   - XTVERSION (`CSI > 0 q`) for terminal name and version
   - DECRQM for modes 2026 (sync output), 2027 (grapheme clusters), 2004 (bracketed paste) and 1004 (focus)
   - kitty keyboard query (`CSI ? u`)
   - a kitty graphics query (`a=q`)
   - OSC 10/11 for foreground and background color (light or dark theme selection)
   - optionally DA2
3. **Result:** a `Caps` struct (color depth, sync, keyboard protocol, mouse modes, graphics protocols, hyperlinks, width policy, `prismattyc_native`). Apps can override it, and it's cached per `TERM_PROGRAM`/version to skip probes on later launches (opt-in).

### 2.8 Input layer

- **Own VT input parser** (Unix first): legacy keys, **kitty keyboard protocol** (progressive enhancement flags: disambiguate, event types incl. release, alternate keys, all-keys-as-escapes, associated text), **SGR mouse** (1006, plus 1000/1002/1003 for buttons, drag and motion; SGR-Pixels 1016 where supported), **bracketed paste** (2004) with streaming for huge pastes, **focus in/out** (1004), resize via SIGWINCH, and OSC/DCS replies routed to the capability system.
- **Windows:** start with VT input mode where available. Fallback is console input records via an existing crate (`crossterm` or `termina`, open decision D3).
- **Events are normalized** into one `Event` enum with timestamps for latency tracing.
- **Ctrl+Shift chords** (the Graphite shortcut overlay) only become unambiguous with the kitty keyboard protocol. With legacy input, the keymap registry automatically offers a documented fallback chord.

### 2.9 Hit-testing and focus (first-class)

- While rendering, every interactive element calls `ui.hit(id, rect, Sense::CLICK | HOVER | SCROLL | DRAG)`. That records `(id, rect, z, clip)` into a per-frame **HitMap**, a flat `Vec` plus a uniform grid index (for example 8×4-cell buckets), giving O(1) average pointer lookup.
- **Pointer routing:** the topmost hit gets the event. Hover enter/leave are synthesized by diffing the hover ID across frames. Drag capture continues outside the rect.
- **FocusManager:** a focus order (document order by default, overridable), focus scopes and traps for modals and overlays, Tab/Shift-Tab and arrow-key spatial navigation (nearest rect in direction), and a `focus_visible` state for styling.
- **Roles** (`Button`, `Tab`, `ListItem`, `TextInput`, …) are stored alongside, ready for a later accessibility export.

### 2.10 Animation and frame scheduler

- **Scheduler states:** `Idle` (blocked on input, 0 wakeups), `Dirty` (render on the next tick, coalescing all pending input first), `Animating` (deadline-driven at the target fps, 60 or 120 or the terminal refresh if known).
- **Frame budget:** if a frame overruns, drop intermediate animation states (time-based animations are frame-rate independent).
- **Animation primitives:** a `Clock` (monotonic, injectable for tests), easing curves, critically damped springs, `Tween<T>` for `Rect`, `Color` (blended in OKLab for nicer gradients) and `f32`, and timelines (sequence, parallel, loop, ping-pong). Animations are keyed by element ID, so they survive re-renders.
- **Buffer effects** (fade, dissolve, sweep, dim) run as post-passes over regions, similar to tachyonfx but marking damage precisely.
- **Determinism:** with a virtual clock, snapshot tests can capture frame N of an animation exactly.

### 2.11 Theme tokens

- **Token registry:** semantic names (`surface.0..3`, `text.primary`, `accent`, `ok`, `warn`, `fail`, `focus.ring`, …) map to a `Color` per profile (truecolor, 256, 16, mono), plus a light/dark variant chosen from the OSC 11 probe.
- **Themes load from TOML or JSON** (serde) and can be hot-reloaded in dev mode.
- **Cells can store `Token` colors** (§2.1), so switching theme re-encodes but doesn't re-render. Otherwise tokens resolve at write time.
- **Style resolution is precomputed:** `Style` handles are resolved once per theme into concrete pen states (no per-cell patch chains).

### 2.12 Backend abstraction

```rust
pub trait Backend {
    fn caps(&self) -> &Caps;
    fn present(&mut self, frame: &FrameDiff<'_>) -> io::Result<PresentStats>; // runs + cursor + ext ops
    fn size(&self) -> io::Result<Size>;
    fn events(&mut self) -> &mut dyn EventSource;                               // optional; runtime can supply its own
}
```

| Backend | Purpose |
|---|---|
| `AnsiBackend` | Default: Unix TTY, and Windows VT where available |
| `TestBackend` | In-memory grid plus an encoder transcript, for snapshots and the vt100 oracle |
| **`PrismattycBackend`** | Two modes: **(a) in-process**: Prismattyc's own GPU renderer consumes the packed grid and damage runs directly, for its chrome; **(b) native protocol**: an app running *inside* Prismattyc negotiates (via XTVERSION identification plus a private query) a side channel (shared-memory ring of packed rows plus damage, with a control socket) and skips ANSI. That enables real alpha translucency, sub-cell offsets, smooth lightcycle motion and pixel-perfect chrome, with automatic fallback to `AnsiBackend` elsewhere |
| Others later | Web (wasm), `wgpu`, image/headless rendering for docs and screenshots |

---

## 3. Workspace, API sketch, threading

### 3.1 Crate layout (`github.com/moonbase2090/lunatui`)

```text
lunatui/                    facade: re-exports, prelude, `run()`
lunatui-core/               Cell, Grid/Surface, Style/Color/Attr, geometry, diff, grapheme slot + ClusterTable, width tables (no_std + alloc)
lunatui-layout/             constraints, closed-form solver, kasuari fallback, Grid/Stack, memo
lunatui-text/               Span/Line/Text (borrowed), measurement, WrapCache, RowSource
lunatui-term/               encoder, Caps probing, session (raw mode/alt screen/restore), input parser, Unix + Windows
lunatui-runtime/            scheduler, Ui context, IDs/memo, HitMap, FocusManager, animation, keymap registry
lunatui-theme/              tokens, profiles, serde themes
lunatui-widgets/            Block, Paragraph, List, Table, Tabs, Gauge, Scrollbar, Spinner, TextInput, Overlay, Canvas (later)
lunatui-test/               styled snapshots, insta integration, virtual clock, vt100 oracle helpers
lunatui-ratatui/            compatibility: ratatui Backend impl + widget bridge (§4)
lunatui-prismattyc/         Prismattyc backend + protocol spec (§2.12)
benches/  fuzz/  xtask/  examples/  docs/
```

Graphite lives in its own repo (§5): `graphite-core` (renderer-agnostic) and `graphite-tui` (on lunatui).

### 3.2 API sketch

A complete small app:

```rust
use lunatui::prelude::*;

struct App { gates: Vec<Gate>, selected: usize, running: bool }

impl lunatui::App for App {
    type Msg = Msg;

    fn update(&mut self, ev: Event, cx: &mut Cx<Msg>) {
        match ev {
            Event::Key(k) if k.is(Key::Char('q')) => cx.quit(),
            Event::Key(k) if k.is(Key::Down)      => { self.selected += 1; cx.redraw(); }
            Event::Click(id) if id.is("rerun")     => cx.spawn(run_gates()), // result arrives as Msg
            _ => {}
        }
    }

    fn view(&self, ui: &mut Ui) {
        let [body, status] = ui.area().rows([fill(1), len(1)]);
        let [rail, main]   = body.cols([len(3), fill(1)]);
        ui.render(Rail::new(&self.gates).animate(self.running), rail);
        ui.memo("table", hash(&self.gates, self.selected), |ui| {
            ui.render(Table::from_source(&self.gates).selected(self.selected).id("gates"), main);
        });
        ui.render(Button::new("Rerun").id("rerun"), status.right(9)); // registers hit + focus automatically
    }
}

fn main() -> lunatui::Result<()> {
    lunatui::run(App::new(), Options::fullscreen().fps_cap(60))
}
```

A custom widget looks familiar to ratatui users:

```rust
pub struct StatusDot { state: State }

impl Widget for StatusDot {
    fn render(&self, area: Rect, cx: &mut RenderCx) {        // &self: no consume; cx = grid + theme + time + hits
        let color = cx.theme().token(match self.state { State::Ok => "ok", State::Fail => "fail", _ => "dim" });
        cx.put_str(area.x, area.y, "●", Style::fg(color));
    }
    fn measure(&self, _avail: Size) -> Size { Size::new(1, 1) } // intrinsic size for Auto layouts
}
```

Inline mode (Scorecard in CI-like runs):

```rust
lunatui::run(App::new(), Options::inline(8).keep_on_exit(true))?; // leaves the final panel in scrollback
```

### 3.3 Threading and async model

- **Core is synchronous and `Send`.** No async in `core`, `layout`, `text` or `term`.
- **Default runtime is 2 threads:** an input thread (blocking read on the tty, parses, timestamps, sends to a channel) and the main/render thread (owns the app state, `Ui`, `Grid` and backend). The render thread blocks on `recv_timeout(next_deadline)`, giving zero wakeups when idle.
- **Background work:** `cx.spawn(fn)` runs on a small thread pool, and the result comes back as a `Msg`. An optional `tokio` feature provides `cx.spawn_async(fut)` and an `EventStream` for apps already on tokio.
- **Optional render offload:** encode and write happen on a writer thread with a double-buffered byte buffer, so a slow terminal never blocks input handling (feature-gated, default off until measured).

---

## 4. ratatui migration path

| Level | What it means | How | Feasibility | Effort |
|---|---|---|---|---|
| **L1: lunatui as a ratatui `Backend`** | Any ratatui app keeps its code and gets lunatui's encoder, sync output, single write and caps | Implement `ratatui_core::backend::Backend` in `lunatui-ratatui`. `draw(iter)` feeds cells into our packed grid, and `flush()` runs our encoder | **High.** The trait is small (breakdown §1.6). It doesn't give core speedups, because ratatui still renders and diffs | S |
| **L2: ratatui widgets inside lunatui apps** | Reuse the ratatui widget ecosystem (tui-tree-widget, ratatui-image, tachyonfx output, …) | `RatatuiWidget<W>` adapter: render `W` into a scratch `ratatui::Buffer` for its rect, then convert cells into our grid (styles mapped, `CellDiffOption::Skip` respected), and memoize when inputs are unchanged | **High for rendering, partial for interaction.** Conversion costs an extra pass over the widget's area (estimate: about a few ns per cell). Stateful widgets work; mouse and focus need manual mapping | M |
| **L3: source-compatible facade** (`use lunatui::ratatui_compat as ratatui`) | Recompile ratatui apps unchanged | Re-implement ratatui's public types | **Low/medium.** `Cell` and `Buffer` expose public fields (`buf.content`, `cell.fg`), which conflicts with the packed layout, and the API surface is large. Not recommended | L |

Recommendation: ship **L1 + L2** (PRs 43–46) and publish a "porting guide" covering the mapping of types and concepts. ratatui is MIT, which is compatible with MPL-2.0; if any ratatui code is adapted, keep its MIT notice in those files.

---

## 5. Relationship to Graphite

- **`graphite-core`** is renderer-agnostic and has no lunatui dependency. It holds:
  - the Graphite token set (palette, spacing, radii-as-glyph-choices, motion curves)
  - component *models* (rail segments, pane chrome, chip/badge/status-dot semantics, tab strip state, shortcut-overlay layout from the keymap registry)
  - lightcycle path and timing
  - translucency rules ("overlay alpha 0.7 over surface.1")

  Prismattyc's `graphite.rs` migrates to consume it after 0.3.0 ships.
- **`graphite-tui`** depends on `graphite-core` and lunatui. It implements those components as lunatui widgets, maps tokens into `lunatui-theme`, uses the HitMap, FocusManager and animation for interactivity, and approximates translucency by blending against the underlying cells. In the Prismattyc native backend it emits real alpha.
- **lunatui stays look-neutral.** Graphite is the flagship optional look: `lunatui = { features = ["graphite"] }` re-exports it, or apps depend on `graphite-tui` directly. That keeps the core adoptable by people who want their own look, while Graphite becomes the showcase.

---

## 6. Benchmark and proof plan

### 6.1 Bench suite (vs ratatui)

- `benches/scenes/`: the dashboard, log, table and gradient scenes ported from `/workspace/ratatui-perf`. Each is implemented **twice**: once on ratatui 0.30.x (pinned) and once on lunatui, rendering identical visible content. A test asserts both produce the same screen text and styles via the vt100 oracle, so the comparison is fair.
- **Metrics per scene:** render, diff, encode, end to end, bytes, `write()` count, allocations (counting allocator), and peak memory.
- **Micro benches:** layout (n = 2…32, each constraint kind, hit and miss), wrap (cold, warm, append), width (ASCII, CJK, emoji), encoder (SGR churn, sparse, dense), input parser (MB/s).
- **Harnesses:** `criterion` for wall-clock locally. **`gungraun`** (callgrind instruction counts; appears to be the continuation of `iai-callgrind`) for **stable CI numbers** on noisy runners. `divan` is optional for quick comparisons.

### 6.2 CI perf regression gates

- On every PR, run `gungraun` instruction counts for about 30 key benches. **Fail if any regress by more than 3%** (instruction counts are deterministic enough for tight gates). Also fail on any allocation-count increase in the zero-alloc paths, or any byte-count increase in the encoder transcripts.
- Nightly, run wall-clock criterion on a dedicated self-hosted runner (fixed CPU governor), and publish trends to a `perf/` page.
- Results are published as a table "lunatui vs ratatui" in the README, regenerated by `xtask bench-report`.

### 6.3 Real-terminal latency and throughput

| What | Method | Terminals |
|---|---|---|
| **Library latency** | Internal tracing: input timestamp → frame write complete. Reports p50/p99 | All |
| **Terminal parse throughput (proxy)** | After writing a frame, send `CSI c` (DA1) and time the reply. That measures "terminal finished parsing up to here", not paint. Replay recorded frame streams (tick, scroll, gradient) | Prismattyc, kitty, WezTerm, Ghostty, Alacritty, iTerm2, (Terminal.app, Windows Terminal) |
| **End-to-end input→photon** | (a) **Prismattyc:** add a latency probe in Prismattyc that timestamps key receipt and GPU present of the frame containing the response (exact, and Prismattyc is ours). (b) Others: screen-capture approach (inject a key via OS automation, poll screen pixels for a change, Typometer-style), and optionally a high-speed camera for a reference run | Same list |
| **Tearing check** | A frame alternates two full-screen patterns, captured at high frame rate, counting mixed frames with and without sync output | Same list |

Results go into `docs/latency.md` with hardware and versions. Anything measured with a proxy is labeled as such.

---

## 7. Testing strategy

The binding rules are in §M. This section lists the concrete harnesses that implement them.

- **Styled snapshots:** `lunatui-test` serializes a grid as a text layer plus run-length style layers (fg/bg/attrs per run, token names when available), stored as `insta` snapshots that are readable in review. Snapshot each Graphite component in every state, and animations at fixed virtual-clock times.
- **vt100 oracle for the encoder:** feed the encoder's byte output into the `vt100` crate's parser (or `alacritty_terminal`) and assert the parsed screen equals our grid. This runs on every snapshot test and in property and fuzz tests. It's the main guard against cursor and SGR optimizations corrupting output.
- **Property tests (`proptest`, M3):**
  - **old screen + encode(diff(old, new)) == new** (the core invariant), for random grids, edits and clusters
  - closed-form layout == kasuari fallback on random non-conflicting constraints, sum-of-sizes == area, monotonicity
  - wrap cache == fresh wrap
  - width(ASCII fast path) == table width
  - SIMD == scalar (§10)
- **Differential vs ratatui (M4):** `tests/differential/` runs scripted scene sequences on both libraries through the vt100 oracle; allowed differences go in `ALLOWED.md`.
- **Fuzzing (`cargo-fuzz`/`libfuzzer-sys`, `arbitrary`, M5):** input parser (arbitrary bytes never panic, round-trip known sequences), encoder (arbitrary grid ops → oracle equality), capability-reply parser, theme TOML loader. Run in CI for a short time per PR, and as a long nightly campaign.
- **Unicode corpus tests:** the Unicode grapheme break test file plus a curated emoji/VS16/ZWJ/CJK/combining corpus, checked against per-terminal expected widths captured from real terminals (`xtask capture-widths` prints test strings and reads the cursor position back).
- **End-to-end:** `termlens` PTY tests for example apps and for Scorecard.
- **Platform matrix:** Linux and macOS (tier 1), Windows (tier 2 until §9 is resolved).

---

## 8. Phased roadmap, PR by PR

Conventions: every PR is small (target ≤ ~600 changed lines excluding generated tables and snapshots), built by one implementer seat, and reviewed per §T. Size **S** is about 1–3 days, **M** about 3–7 days of focused work (estimates). **DD-n** rows are design-doc PRs (M7), counted in the PR totals.

### 8.0 Per-PR acceptance criteria template (applies to every PR in addition to its row's "Accept")

```markdown
## Acceptance checklist
- [ ] Roadmap item: PR __  | Issue: #__  | ADR: DD-__ (required for cell format, layout engine, public API)
- [ ] M1 Tests-first: red commit <sha> linked; CI `tests-first` job shows ≥1 new test failing on base (or label `no-tdd` with reason)
- [ ] M2 Benchmarks: bench exists before optimization; target stated: ____ ; CI perf gate green (no bench > 3% slower than main);
      alloc counts and encoder byte counts not increased
- [ ] ratatui comparison table included (ratatui | main | this PR | Δ | target) for every touched bench/scene
- [ ] M3 Property tests: invariants covered: ____ (diff-apply invariant if grid/diff/encoder touched)
- [ ] M4 Differential vs ratatui: identical vt100 end state for affected scenes (or ALLOWED.md entry, reviewed)
- [ ] M5 Fuzz: targets touched ____ ran ≥60 s in CI with no findings (input parser / encoder / caps / theme)
- [ ] M6 Styled snapshots added or updated for visual changes; each snapshot diff explicitly accepted
- [ ] M8 Dogfood: Scorecard branch builds against this PR (from PR 10 on), friction logged as `dogfood` issues
- [ ] Docs on all new public items; CHANGELOG entry
- [ ] T5 Authorship: all commits authored/committed by mb2090 noreply; no Co-authored-by trailers (CI `authorship` green)
- [ ] T3 Muse: `SHIP <head-sha>` posted for the current head
- [ ] T4 Opus checkpoint resolved (only for CP-1…CP-4 PRs)
```

### Phase 0: Foundations

| PR | Title | Scope / files | Tests | Accept |
|---|---|---|---|---|
| 01 | Workspace scaffold | Cargo workspace, crates stubbed, MPL-2.0 `LICENSE` + headers, `deny.toml`, CI (fmt, clippy, test on Linux/macOS/Windows), MSRV pin | CI itself | CI green on 3 OSes; `cargo deny` passes |
| 02 | Bench harness | `benches/` with criterion + gungraun, counting allocator, byte/write counters | Harness self-test | `xtask bench` runs and prints metrics |
| 03 | ratatui baseline scenes | Port `ratatui-perf` scenes (dashboard, log, table, gradient) on pinned ratatui | Scenes render | Numbers within ±15% of `results.txt` on the same box |
| 04 | Unicode width tables | `xtask gen-width` from UCD + emoji data → `lunatui-core/src/width/tables.rs`; `width(&str)` | Corpus + diff vs `unicode-width` (documented deltas) | All differences explained in `docs/width-policy.md` |
| DD-1 | **Design doc: cell format** | `docs/adr/0001-cell-format.md`: 16-byte cell, grapheme slot, per-frame cluster table + ID seeding, continuation cells, mode 2027 policy, ExtTable, benchmarks | Prototype bench (from `ratatui-perf`) attached | Muse review, **Opus CP-1**, MB2090 sign-off |
| 05 | Grapheme slot + cluster table | `core/src/grapheme.rs`: inline scalar, tagged `ClusterId`, per-frame `ClusterTable` with seeding from the previous frame | Proptest: round-trip; ID stability across frames | Zero allocations for inline glyphs; implements DD-1 |

### Phase 1: Grid and encoder

| PR | Title | Scope / files | Tests | Accept |
|---|---|---|---|---|
| 06 | Packed `Cell` + style types | `core/src/{cell,color,attr,style}.rs`, 16 B static assert | Unit + size asserts | `size_of::<Cell>() == 16` |
| 07 | `Grid` + write APIs + dirty rows | `core/src/grid.rs`: `put_str`, `fill`, `set_style`, row dirty spans, continuation-cell rules; internal styled-snapshot serializer | Unit + proptest (writes never break lead/continuation pairs) | Writes O(len), dirty spans exact |
| 08 | Scalar diff → runs | `core/src/diff.rs` | Proptest vs naive diff | Unchanged 200×60 ≤ 8 µs (gungraun baseline recorded); **Opus CP-1 follow-up review** |
| 09 | Encoder v1 | `term/src/encode/{sgr,cursor,mod}.rs`: pen-state delta, combined CSI, cursor cost model, single buffer | Transcript snapshots; grid-level differential vs ratatui `CrosstermBackend` (M4) | Bytes ≤ ratatui on all scenes; identical oracle end state |
| 10 | Session + sync output (Unix) | `term/src/session.rs`: raw mode, alt screen, restore on drop/panic, 2026 wrapping, one `write_all` per frame | PTY test (termlens) | 1 write per frame; restore after panic |
| 11 | vt100 oracle + diff-apply property + encoder fuzz | `test/src/oracle.rs`, `fuzz/fuzz_encoder.rs`, `tests/differential/` harness | Proptest (old + encode(diff) == new) + fuzz | 10M fuzz execs clean; oracle on all snapshots |
| 12 | Capability probe | `term/src/caps.rs`: env + batched queries (incl. DECRQM 2026/2027) + DA1 sentinel + timeout; enable 2027 when reported | Fake-terminal replies; PTY tests | Probe ≤ 200 ms worst case; correct on recorded replies from 6 terminals |
| 13 | Color profiles + downsampling | `theme/src/profile.rs`: truecolor→256→16→mono (perceptual nearest) | Golden mappings | Gradient scene readable at 16 colors (snapshot) |

### Phase 2: Layout and text

**Implementer seat B joins around PR 15** (§T1). From here, seat A takes layout and text and seat B starts the input and runtime track (PRs 20–22), each one PR at a time.

| PR | Title | Scope / files | Tests | Accept |
|---|---|---|---|---|
| DD-2 | **Design doc: layout engine** | `docs/adr/0002-layout-engine.md`: constraint semantics, closed-form algorithm, conflict detection, kasuari boundary, ratatui parity, memo, `Grid`/`Stack`/`Auto` | Parity test plan + bench targets | Muse review, **Opus CP-2**, MB2090 sign-off |
| 14 | Closed-form layout | `layout/src/{constraint,flex,solve_fast}.rs` | Unit + proptest invariants | 20 × `Fill` miss ≤ 20 µs |
| 15 | Solver fallback + equivalence | `layout/src/solve_cassowary.rs` (kasuari), conflict detection | Proptest fast == fallback; parity suite vs ratatui | Documented, intentional differences only |
| 16 | Layout memo + `Grid`/`Stack` + `Auto` | `layout/src/{memo,grid,stack}.rs` | Unit | Zero allocations on memo hit; **Opus CP-2 follow-up review** |
| 17 | Borrowed text + frame arena | `text/src/{span,line,text,arena}.rs` | Unit + alloc counter | Dashboard text building: 0 heap allocations |
| 18 | WrapCache | `text/src/wrap/{mod,cache,index}.rs` | Proptest cached == fresh | 5k-line wrapped scroll ≤ 0.5 ms at 400×120 |
| 19 | `RowSource` virtualization | `text/src/source.rs` | Unit | 1M-row table renders visible rows only (constant time vs length) |

### Phase 3: Runtime

| PR | Title | Scope / files | Tests | Accept |
|---|---|---|---|---|
| 20 | Input parser | `term/src/input/{parser,keys,mouse,paste}.rs`: legacy + kitty keyboard + SGR mouse + paste + focus | Table tests from real captures; `fuzz_input_parser`; `parse(encode(e)) == e` property | Fuzz clean; all captured sequences decode |
| 21 | Input thread + coalescing | `runtime/src/input.rs` | Unit with fake source | Bursts coalesce to one frame |
| 22 | Frame scheduler | `runtime/src/sched.rs`: Idle/Dirty/Animating, deadlines, backpressure | Virtual clock tests | 0 wakeups idle (measured with a counter); frame-drop under a slow writer |
| DD-3 | **Design doc: public API** | `docs/adr/0003-public-api.md`: `App`, `Ui`, `Widget`, `RenderCx`, IDs, memo, `Cx` messaging, threading, error model, semver policy | API mock-up compiled as doc tests; Scorecard dogfood sketch | Muse review, **Opus CP-3**, MB2090 sign-off |
| 23 | `App`/`Ui`/`run()` | `runtime/src/{app,ui,id,memo}.rs`, facade `run()` | Example app PTY test | Hello-world + counter examples work |
| 24 | HitMap + FocusManager | `runtime/src/{hit,focus}.rs` | Unit + proptest (topmost wins) | Click/hover/drag/Tab/arrow traversal work in the example |
| 25 | Animation primitives | `runtime/src/anim/{clock,ease,spring,tween}.rs` | Deterministic snapshots at time t | Frame-rate-independent results |
| 26 | Theme tokens | `theme/src/{token,theme,serde}.rs`, light/dark from OSC 11 | Unit + snapshot | Theme switch re-encodes without re-render |
| 27 | Windows backend (tier 2) | `term/src/sys/windows.rs` (VT output, input per D3) | CI on Windows; manual Windows Terminal check | Examples run in Windows Terminal |

### Phase 4: Widgets, testing kit, first release

| PR | Title | Scope / files | Tests | Accept |
|---|---|---|---|---|
| 28 | `Widget` trait + `RenderCx` | `widgets/src/lib.rs` (`render(&self, Rect, &mut RenderCx)`, `measure`) | Unit | Trait documented; matches DD-3; **Opus CP-3 follow-up review** |
| 29 | Block, borders, titles, symbols | `widgets/src/block.rs`, `core/src/symbols/` | Styled snapshots; differential vs ratatui `Block` | Identical end state to ratatui Block |
| 30 | Paragraph/TextView | `widgets/src/paragraph.rs` on WrapCache | Snapshots + bench | Log scene meets the §1.3 target |
| 31 | List + Table (virtualized) | `widgets/src/{list,table}.rs` on RowSource | Snapshots + bench | Table scene meets target; selection via focus |
| 32 | Tabs, Gauge, Scrollbar, Spinner, StatusBar, TextInput | `widgets/src/*` | Snapshots | Dashboard scene fully on lunatui |
| 33 | Overlay/backdrop + blend | `widgets/src/overlay.rs` (focus trap, translucency by blending) | Snapshots | Modal traps focus; blend snapshot |
| 34 | Publish `lunatui-test` crate | Extract the internal styled-snapshot serializer (PR 07), oracle (PR 11), virtual clock (PR 25) into a public crate | Self-tests | Used by all widget tests and by Scorecard |
| 35 | Docs, examples, release **v0.1.0** | `docs/`, `examples/`, README perf table vs ratatui, porting notes | Doc tests | **Opus CP-4 pre-v0.1 review resolved**; **first usable release** |

**First usable release = PR 35 in code-PR numbering, which is the 38th PR overall (35 code PRs + DD-1, DD-2, DD-3) → v0.1.0.**

### Phase 5: Graphite and Scorecard (first real consumer)

Scorecard dogfooding starts at PR 10 and becomes a live branch at PR 23 (M8). These PRs graduate that work into the Graphite and Scorecard repos.

| PR | Title | Scope | Tests | Accept |
|---|---|---|---|---|
| 36 | `graphite-core` extraction | Tokens, component models, motion curves (no renderer deps) | Unit | Prismattyc compiles against it (behind a flag) |
| 37 | `graphite-tui` chrome | Rail, Pane, Chip, Badge, StatusDot, TabStrip as lunatui widgets | Styled snapshots | Visual sign-off by MB2090 |
| 38 | Keymap registry + shortcut overlay + palette | Ctrl+Shift overlay (kitty kbd, legacy fallback), command palette | Snapshots + PTY | Overlay and palette driven by one registry |
| 39 | Lightcycle + sub-cell raster | Eighth/octant glyph raster, trail gradient | Virtual-clock snapshots | Smooth at 60 fps, ≤ 1% CPU (target) |
| 40 | Scorecard TUI skeleton | `scorecard tui`: rail of gates, tabs with badges, status bar | termlens flow | Runs in kitty/WezTerm/Prismattyc |
| 41 | Scorecard findings + inline mode | Virtualized findings table, OSC 8 `file:line` links, `--inline` summary | Snapshots | 100k findings scroll smoothly |
| 42 | Scorecard tests + perf gates + release | Snapshots, termlens, perf budget in CI | CI | **Scorecard ships on lunatui** |

**Scorecard adoption = PR 42 in code-PR numbering, which is the 45th PR overall.**

### Phase 6: ratatui compatibility

| PR | Title | Scope | Tests | Accept |
|---|---|---|---|---|
| 43 | L1: ratatui `Backend` | `lunatui-ratatui/src/backend.rs` | Run 5 ratatui examples via oracle | Identical screens; byte reduction reported |
| 44 | L2: widget bridge | `RatatuiWidget<W>` with buffer conversion + memo | Snapshots of 10 ratatui widgets | Visual parity |
| 45 | Style/Text conversions | `From` impls both ways | Unit | Lossless for the common subset |
| 46 | Porting guide + 3 ported examples | `docs/porting-from-ratatui.md` | Examples | Guide reviewed |

### Phase 7: SIMD and targeted asm (§10)

Under M2, each of these PRs lands its benchmark and numeric target *before* the optimized code (a separate earlier commit or PR), and its scalar path stays the property-test reference (M3).

| PR | Title | Scope | Tests | Accept |
|---|---|---|---|---|
| 47 | Dispatch framework | `core/src/simd/{mod,detect}.rs`: scalar reference, runtime dispatch (x86_64: SSE2 baseline, AVX2 detected; aarch64: NEON) | Differential proptest harness | Zero behavior change |
| 48 | SIMD row diff | Find first/last mismatch per dirty span with 128/256-bit compares + movemask (NEON equivalent) | Differential + fuzz | ≥ 1.5× on the diff bench on 2 machines, or reverted |
| 49 | SIMD ASCII/width fast path | Validate ASCII runs 16/32 bytes at a time → width = len | Differential | ≥ 1.3× on the text bench, or reverted |
| 50 | Encoder fast paths | ASCII run copy, SGR parameter formatting tables, optional SIMD scan for escape-free runs | Oracle + fuzz | ≥ 1.2× encode throughput, or reverted |
| 51 | Inline-asm experiments (optional) | Only where intrinsics leave a proven gap | Differential + Miri on the scalar path | ≥ 10% over intrinsics on 2 machines, or not merged |

### Phase 8: Prismattyc native backend

| PR | Title | Scope | Accept |
|---|---|---|---|
| 52 | In-process grid sink | `lunatui-prismattyc`: Prismattyc chrome renders from the lunatui grid + damage | Prismattyc chrome draws via lunatui |
| 53 | Native protocol spec + negotiation | Private query, shared-memory ring format, versioning, security model (same-user only) | Spec reviewed; fallback tested |
| 54 | Prismattyc-side implementation | In the Prismattyc repo | Scorecard runs natively inside Prismattyc |
| 55 | Real alpha + sub-cell in native mode | Graphite translucency and lightcycle use native features | Visual sign-off; ANSI fallback identical in layout |

### Phase 9: SkillSaw and 1.0

| PR | Title | Accept |
|---|---|---|
| 56 | SkillSaw TUI on lunatui + Graphite | Second real app shipped |
| 57 | API review + stabilization pass | Breaking-change list closed |
| 58 | Accessibility role export (experimental) | Roles from the HitMap exported (format TBD) |
| 59 | Published performance and latency report | §6.3 results for 6+ terminals |
| 60 | **v1.0.0** | Windows tier 1, stable API, perf gates enforced |

### Public announcement milestones

| Milestone | When | What makes it credible |
|---|---|---|
| **M1: "Show HN"-style preview** | After **PR 35 (v0.1; 38th PR overall)** | Usable crate, docs, published vs-ratatui numbers. Frame it as a preview, without "go-to" claims |
| **M2: Main public launch** | After **PR 42 + PR 43 (45th–46th PR overall)** | A real shipped app (Scorecard), ratatui apps can use the backend today, perf gates in CI |
| **M3: "Run your ratatui widgets"** | After **PR 46** | Porting guide plus bridge |
| **M4: 1.0** | **PR 60** | Stability, Windows tier 1, latency report |

---

## 9. Risks and mitigations

| Risk | Why it's real | Mitigation |
|---|---|---|
| **Scope creep** | This is a terminal library, a layout engine, a text engine, a runtime and widgets | Strict phases. v0.1 has a fixed widget list. Graphite and Scorecard drive priorities. Anything not needed by Scorecard or SkillSaw waits until after 1.0 |
| **Unicode / grapheme correctness** | ratatui has a long tail of width bugs (#1271, #2063, #2651, #2686 …) | Own tables, a terminal width policy, a mode-2027 probe, a captured per-terminal width corpus, the vt100 oracle, and fuzzing |
| **Terminal quirks** | Background-color-erase, `REP`/`ECH` support, sync output, kitty protocol coverage all vary | Every optimization is capability-gated with a safe fallback. A per-terminal quirks table comes from probes and captures. "Conservative mode" is a single switch |
| **Windows** | Console APIs differ, VT input availability varies by version, and there's limited local expertise | Tier 2 at v0.1 (using existing crate input per D3), VT output where available, Windows CI from PR 01, tier 1 only at 1.0 |
| **Performance claims don't hold** | Targets are partly extrapolated | PR 02–03 baselines come first. Every perf PR shows before and after. Targets are revised publicly if the data says so |
| **Ecosystem adoption** | ratatui is entrenched, with a large widget ecosystem and community | L1/L2 compatibility, a porting guide, real apps, generous docs, responsive issue triage. Position it as complementary (ratatui apps can use the backend), not hostile |
| **License friction** | MPL-2.0 is file-level copyleft; some companies prefer MIT/Apache for UI dependencies | MPL allows use from proprietary code; only modified MPL files must be shared. Explain this clearly in the README FAQ |
| **Bus factor** | Mostly one founder | Small PRs, ADRs in `docs/adr/`, recruit co-maintainers at M2 |
| **Process overhead slows delivery** | §M/§T add a red commit, benches, differential runs and reviews to every PR | Templates and `xtask pr-check` automate the checklist; the `no-tdd` label for docs/CI PRs; fuzz and differential jobs scoped to touched components |
| **Flaky perf gate** | 3% wall-clock thresholds are noisy on shared CI | Gate on `gungraun` instruction counts (deterministic); wall-clock reported, not gated, except on the dedicated nightly runner |
| **Memo-key bugs** (incremental core) | Stale UI if a key misses state | `LUNATUI_VERIFY` mode, docs, lint-style debug assertions |
| **Unsafe code** (SIMD/asm, arena) | Memory-safety bugs | Scalar reference, differential tests, Miri on scalar paths, `unsafe` confined to `core::simd` and the grapheme slot module, `#![forbid(unsafe_code)]` elsewhere |

---

## 10. Assembly vs Rust: recommendation

**Recommendation:** write everything in Rust. Add **targeted SIMD** via `std::arch` intrinsics with **runtime per-architecture dispatch** and **scalar fallbacks**, in the three loops profiling identifies as hot:
- **row diff:** find mismatch spans across 16-byte cells
- **width lookup:** ASCII and short-run validation, with tables for the rest
- **output encoding:** ASCII run copying, SGR number formatting, escape-free run scanning

Use `core::arch::asm!` (stable) **only** where intrinsics measurably fall short. This is Phase 7 (PRs 47–51), deliberately *after* the algorithmic wins.

- **Dispatch:** on x86_64, SSE2 is the baseline and AVX2 is detected at runtime via `is_x86_feature_detected!`, with function pointers cached once. On aarch64, NEON is the baseline. Everything else (RISC-V, wasm32, others) uses scalar, and wasm `simd128` is optional later.
- **Portable SIMD (`std::simd`)** is still nightly-only, so it lives behind an opt-in `nightly` feature for experimentation, never as the default.
- **Gates:** each SIMD or asm PR must show ≥ X% improvement on two machines (one x86_64, one Apple Silicon or other aarch64) in both criterion and instruction counts (thresholds per PR in §8). It must pass differential property tests (SIMD == scalar on random inputs) and fuzzing. If it misses the gate, it's reverted. `unsafe` is confined to one module with documented invariants.

**Why not a full-assembly core:**
1. **The bottlenecks are algorithmic and I/O, not instruction-level.** The measured worst cases are an O(n²) solver (11.3 ms), an O(offset) re-wrap (10.4 ms), a 48-byte cell layout, and 67 writes per frame. Asm fixes none of these; data structures do.
2. **Portability.** Separate cores for x86_64, aarch64, RISC-V and wasm, multiplied across OS ABIs, is an enormous maintenance load for gains the compiler and intrinsics already capture.
3. **Safety and correctness.** No borrow checker, no Miri, harder fuzzing triage. A TUI library is linked into other people's programs.
4. **Optimizer interference.** Asm blocks are opaque to inlining, LTO and auto-vectorization around them, which often makes the surrounding code slower.
5. **Contributors and auditability.** Assembly sharply narrows who can review and maintain the code, which works against becoming the go-to library.
6. **The terminal emulator is usually the final bottleneck** (parsing and painting). Bytes saved by the encoder matter more than nanoseconds saved in Rust.

---

## 11. Open decisions for MB2090 (with recommendations)

| # | Decision | Options | Recommendation | Status |
|---|---|---|---|---|
| D1 | **Name** | §12 candidates | **`lunatui`** (free on crates.io and GitHub, on-brand). This PRD already uses it; confirm or rename before PR 01 | Approved |
| D2 | **Core model** | Immediate / retained / immediate API + incremental core | **Immediate API + incremental core** (§2.2) | Approved |
| D3 | **Terminal I/O layer** | Own everything / build on crossterm / build on termina | **Own VT output and Unix input**. On Windows, reuse `crossterm` (mature Windows console input) until our own Windows input is proven | Approved |
| D4 | **Cell format** | 16 B packed + ext table / 8 B style-ID | **16 B with the 4-byte grapheme slot and per-frame cluster table (§2.1)**, finalized in DD-1 | Approved |
| D5 | **Token colors stored in cells** | Yes (theme switch without re-render) / resolve at write | **Yes, behind a feature flag at first.** Measure the encode-time cost | Approved |
| D6 | **Async** | Sync core + optional tokio adapter / async-first | **Sync core + optional tokio feature** | Approved |
| D7 | **MSRV / edition** | Latest stable / N-2 / match ratatui (1.88) | **Edition 2024, MSRV 1.88**, raised deliberately with notice | Approved |
| D8 | **`no_std` core** | Yes / no | **Yes for `core`, `layout`, `text`** (cheap now, expensive later) | Approved |
| D9 | **Windows tier** | Tier 1 at v0.1 / tier 2 until 1.0 | **Tier 2 until 1.0** | Approved |
| D10 | **ratatui compat depth** | L1 / L1+L2 / L1+L2+L3 | **L1 + L2** | Approved |
| D11 | **Graphite coupling** | Graphite inside core / optional feature / separate only | **Optional `graphite` feature re-export; Graphite in its own repo** | Approved |
| D12 | **Prismattyc native protocol** | Shared memory + socket / escape-sequence only / defer | **Spec in Phase 8.** Keep the in-process sink first (it also serves Prismattyc's own chrome) | Approved |
| D13 | **Public launch timing** | At v0.1 / at Scorecard adoption | **Preview at v0.1; main launch at PR 42 + 43** | Approved |
| D14 | **SIMD gate thresholds** | Fixed % per PR | **As in §8 Phase 7** (1.2–1.5× for intrinsics, +10% over intrinsics for asm) | Approved |
| D15 | **Governance** | Solo / co-maintainers at M2 | **Invite 1–2 co-maintainers at M2**, ADRs from day one | Approved |
| D16 | **mb2090 noreply ID** | Resolved | `322824348+mb2090@users.noreply.github.com` (used by T5 and the `authorship` CI job) | Resolved |
| D17 | **Muse tooling** | Muse runs benches itself / reads CI artifacts only | **Read CI artifacts, with re-run allowed on demand.** Keeps "blind" honest without doubling CI cost | Approved |

Approved by MB2090 on 2026-10-06: all recommendations accepted.

---

## 12. Name candidates (checked Oct 5, 2026)

Checks used the crates.io API (`/api/v1/crates/<name>`: 404 means free) and an HTTP check of `github.com/<name>` (200 means an account or org exists). crates.io treats `-` and `_` as the same name.

| Name | crates.io | GitHub `github.com/<name>` | Notes |
|---|---|---|---|
| **`lunatui`** (recommended) | **free** (also free: `lunatui-core`, `lunatui-widgets`, `lunatui-crossterm`, `luna-tui`) | **free** (404) | Moonbase lunar theme, "tui" helps discovery, sits alongside ratatui without copying it. Minor risk: "luna" can suggest Lua |
| `moontui` | free (also `moontui-core`) | free (404) | Very on-brand, plain. Less distinctive |
| `hypercell` | free (also `hypercell-core`) | **taken** (an account exists; type unclear) | Says "hyperspeed + cells". The "hyper" prefix is crowded in Rust (the `hyper` HTTP ecosystem) |
| `cellfire` | free | **taken** (user "Cellfire (Lim Jin Hann)") | Energetic and distinctive. No "tui" in the name |
| `gridfire` | free | **taken** (account exists) | Similar to cellfire |
| `warpgrid` | free | **taken** (org "WarpGrid") | "Warp" collides with the Warp terminal brand and the `warp` HTTP crate. Avoid |
| `hyperspace-tui` | free (`hyperspace` alone also free) | `hyperspace` is **taken** (org "Hyperspace") | Matches "hyperspeed". Generic, with a long crate name |
| `tachytui` | free | free (404) | Speed connotation. Too close to `tachyonfx` (ratatui org). Avoid |

Taken on crates.io (for reference): `orbit-tui`, `glint`, `tessera`, `lumen-tui`, `apogee`, `perigee`, `lightcycle`, `mooncell`.

Repos use `github.com/moonbase2090/<name>`, so a taken GitHub account only matters for brand confusion and vanity URLs. `github.com/moonbase2090/lunatui` is currently free.

Recommendation: decide the name, then publish a real (if minimal) `0.0.x` from PR 01 to hold it. crates.io's usage policy discourages pure name-squatting, so ship working code, not a placeholder.

---

## Appendix: what's measured vs estimated

- **Measured** (ratatui v0.30.2, `/workspace/ratatui-perf/results.txt`): every number in the "ratatui" column of §1.3, plus the 16-byte packed-cell diff prototype (4.2 µs) and the `write()` counts (67 vs 1).
- **Targets:** every number in the "lunatui target" column.
- **Estimates:**
  - PR sizes, the encoder byte savings, and animation CPU
  - capability probe timing
  - L2 bridge cost
  - which terminals support DEC 2026 and 2027: these must be probed and captured during PR 12. I didn't verify terminal support lists for this plan.
- **Library choices:** `gungraun` being the successor of `iai-callgrind` is my reading of crates.io metadata (both are callgrind-based, and gungraun is the actively updated one). Confirm before adopting.

## M9 Small pieces, incremental growth (binding, MB2090 Oct 5, 2026)
- Build from small, independently testable units; each PR adds or improves one small piece.
- No god objects: no type or module that owns unrelated responsibilities. Prefer small focused types and free functions composed together.
- Soft limits: a source file over ~400 lines or a type with more than ~7 fields or ~10 public methods must be split or justified in the PR body; Muse checks this before SHIP.
- Grow features incrementally: the simplest working version first, behind tests, then optimize in follow-up PRs with benchmarks.

## M10 Code beauty (binding, MB2090 Oct 5, 2026)
"The codebase should be as pretty as the performance is fast."
- Readable first: clear names, small functions, one idea per module, no clever tricks without a comment explaining why.
- Unsafe code and SIMD are isolated in tiny, documented modules with a safe API and a scalar reference implementation alongside.
- Every public item has rustdoc with an example; `#![deny(missing_docs)]` on public crates; doc examples are tested.
- Tooling gates in CI: `cargo fmt --check`, `cargo clippy -- -D warnings` (pedantic where sensible), `cargo doc` with no warnings, `cargo deny`, and typos checks.
- Consistent error handling (typed errors, no panics in library code except documented invariants).
- Muse reviews readability and API elegance as a SHIP criterion alongside correctness and benchmarks.

## M11 No shortcuts (binding, MB2090 Oct 5, 2026)
"Readable, highly maintainable code. Do not take shortcuts."
- No TODO/FIXME/`unimplemented!`/`todo!` merged to main without a linked issue; no commented-out code.
- No disabling or skipping tests, lints, or benchmarks to get a PR green; no `#[allow]` without a written reason.
- No copy-paste duplication; extract shared logic.
- No `unwrap`/`expect` in library code outside tests and documented invariants.
- Fix root causes, not symptoms; workarounds require an issue and a comment.
- Muse rejects any PR that cuts a corner, even if tests and benchmarks pass.

## M12 Regular housekeeping (binding, MB2090 Oct 5, 2026)
Optimization and refactoring are a standing, recurring task, not a one-off phase.
- A housekeeping pass is scheduled regularly (e.g., after every ~5 feature PRs, or weekly): profile hot paths, review the benchmark trend, and refactor anything that grew too large or tangled under M9–M11.
- Housekeeping PRs follow every rule above (tests first, benchmarks, small scope, proof in the body) and never mix with feature work.
- Findings that can't be fixed in one pass become tracked issues labeled `housekeeping`.

## T-update Seat assignments (MB2090 Oct 5, 2026)
- Implementer: Claude Opus 5.5.
- Reviewer (every PR, blind, SHIP <sha>): a dedicated Muse seat (muse-spark-1.3-contributor, xhigh), a different model family from the implementer.
- Checkpoint reviewer (CP-1 to CP-4: cell format, layout engine, public API, pre-v0.1): Codex, replacing Claude Opus since Opus now implements.
