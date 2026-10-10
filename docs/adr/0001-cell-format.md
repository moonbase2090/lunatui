# ADR 0001: Cell format (DD-1)

| | |
|---|---|
| **Status** | Proposed. Approval chain (PRD M7, §T4): Muse review, then the CP-1 checkpoint review (Codex, per the seat assignments), then MB2090 sign-off |
| **Date** | 2026-10-06 |
| **Decides** | PRD D4 (cell format), DD-1 |
| **Implemented by** | PR 05 (grapheme slot and cluster table), PR 06 (`Cell` and style types), PR 07 (`Grid` and dirty rows), PR 08 (diff) |
| **Evidence** | The prototype in `benches/src/prototype.rs`: `cargo run --release --example cell_prototype` and `cargo bench -p lunatui-bench --bench criterion_cell_prototype` |

## 1. Problem

A terminal frame is a grid of cells. Every frame lunatui writes cells, diffs the grid against the previous frame, and encodes the changed runs (PRD §2). The cell format decides how fast the diff is, how much memory a grid takes, whether writes allocate, and how graphemes, wide characters, colors and rare attributes are represented. It is hard to change later: every grid, diff, encoder and widget API depends on it.

ratatui's cell is 48 bytes, with a `CompactString` symbol. Diffing compares cells through that string. The PRD 03 baseline on this Mac takes **63 µs to diff two identical 200×60 grids** (ratatui-perf: 122 µs on its Xeon), and the dashboard allocates 978 times per frame.

## 2. Decision drivers

From PRD §1.3:
- Diff of an unchanged 200×60 grid **≤ 8 µs** scalar, ≤ 4 µs with SIMD.
- **16 bytes per cell**: two 200×60 grids in 375 KiB.
- **Zero allocations** per frame in core in steady state.
- Correct Unicode: grapheme clusters, wide characters, terminal width policy (§2.5, `docs/width-policy.md`).
- **Frame budget:** maximum frame time ≤ 8 ms on a dedicated machine, 16 ms hard gate in CI.
- Theme switches without re-rendering, behind a feature flag at first (D5).

## 3. Options considered

| Option | Size | Equality | Writes | Verdict |
|---|---|---|---|---|
| **A. ratatui-style cell:** `CompactString` symbol, `Style` fields | 48 B | string compare per cell | allocate for symbols longer than 24 bytes; per-cell style patching | Rejected: what we are replacing. 63 µs to diff an identical 200×60 grid. |
| **B. 16-byte packed cell plus a per-frame grapheme cluster table and a rare-data table** | 16 B | one 128-bit compare; whole rows with `memcmp` | no allocation for single-codepoint graphemes; clusters interned | **Chosen** (D4, recommended in PRD §2.1). |
| **C. 8-byte cell:** glyph `u32` plus an interned style ID `u32` | 8 B | one 64-bit compare | every styled write needs a hash and an intern of the style | Rejected: half the memory, but a hash on the hot write path, and a style table that must stay stable across frames for the diff. |
| **D. Separate planes:** one array each for glyph, fg, bg, meta | 16 B per cell over four arrays | four compares per cell | as B | Rejected: a cell write touches four cache lines instead of one, and row equality needs four comparisons. |

## 4. Decision

### 4.1 The cell: four 32-bit fields, stored as one 128-bit word

```text
bit   127 ................ 96 95 .............. 64 63 .............. 32 31 ............... 0
      |        meta          |        bg          |        fg          |       glyph       |
```

`Cell` is a `Copy` value type with typed accessors and constructors. The **grid stores each row as `[u128]`**, and `Cell` converts to and from its `u128` word without `unsafe` (`Cell::from_bits`, `Cell::to_bits`).

The prototype shows why the storage type matters. Comparing identical 200×60 grids on this Mac (Apple M5 Max, release build, `cell_prototype` demo):

| Row storage | Time | Why |
|---|---|---|
| ratatui `Cell` | 63.0 µs | string compare per cell |
| struct of four `u32` fields, derived `PartialEq` | 7.3 µs | field-by-field compare per cell |
| `[u32; 4]` per cell | 5.4 µs | |
| `#[repr(transparent)]` newtype over `u128` | 6.2 µs | still per element |
| **raw `[u128]`** | **3.7 µs** | slice equality of a primitive type compiles to `memcmp` |

The standard library compares a slice with `memcmp` only for primitive element types. A newtype, even a transparent one, falls back to element-by-element comparison. So rows are `[u128]`, and the diff compares rows as slices before looking at cells.

### 4.2 Glyph slot (bits 0–31)

| Value | Meaning |
|---|---|
| bit 31 = 0, value ≤ U+10FFFF | One Unicode scalar value, stored inline: ASCII, Latin, CJK, box drawing, braille, single-codepoint emoji. No lookup and no allocation. |
| `CONTINUATION` = `0x0011_0000` (bit 31 = 0) | The trailing cell of a wide grapheme. It is the first value above U+10FFFF, so it can't collide with a scalar or a cluster ID. |
| bit 31 = 1 | A `ClusterId` (bits 0–30) into the frame's cluster table, for multi-codepoint grapheme clusters: ZWJ sequences, VS15/VS16 sequences, flags, combining sequences, Hangul jamo sequences. |

A blank cell is U+0020 with default colors and no attributes, so a cleared row is all one repeated word.

### 4.3 Cluster table, per frame, with ID seeding

- `ClusterTable` maps a cluster string to a `ClusterId` and caches the cluster's width.
- **ID stability:** at the start of frame N, the table is seeded from frame N−1's live entries, keeping their IDs. New clusters get fresh IDs. Entries unreferenced in frame N are dropped when frame N+1 is seeded. Equal clusters therefore have equal IDs in consecutive frames, so the diff compares glyph slots as integers and never touches strings.
- **Bounded:** at most 65,536 IDs per frame. On overflow the frame re-interns everything and does a full diff. The output is never wrong, only slower.
- **Allocation:** cluster strings are copied into a frame arena that is reused across frames, so steady-state frames don't allocate. Clusters are rare in typical UIs.

### 4.4 Colors (fg bits 32–63, bg bits 64–95)

```text
bit  31 30 | 29 ....... 24 | 23 ................................ 0
     kind  |   reserved    |              payload
```

| Kind | Payload |
|---|---|
| `00` Default | 0 (the terminal's default color) |
| `01` Indexed | 0–255 (0–15 are the ANSI colors) |
| `10` Rgb | `r << 16 \| g << 8 \| b` |
| `11` Token | theme token ID (up to 2²⁴), resolved at encode time (D5, behind a feature flag at first) |

Reserved bits are always zero, so equal colors have equal bits.

### 4.5 Meta (bits 96–127)

| Bits | Field |
|---|---|
| 0–11 | Attributes: bold, dim, italic, underline style (3 bits: none, single, double, curly, dotted, dashed), blink, reverse, hidden, strikethrough, overline. One bit is spare. |
| 12–13 | Width: 0 (continuation), 1 or 2 |
| 14–15 | Flags: skip (the encoder leaves this cell alone, for image placements), protected (reserved) |
| 16–31 | `ExtIndex` into the frame's `ExtTable` (0 = none) |

### 4.6 ExtTable: rare data out of line

Underline color, OSC 8 hyperlink ID and image placement live in a per-frame `ExtTable`. A cell refers to an entry by its 16-bit index, so the 99% case stays in 16 bytes. Entries are deduplicated within a frame and seeded across frames like clusters, so equal rare data has equal indices and the diff stays an integer compare. Index 0 means no extra data.

### 4.7 Width and wide graphemes

- Width is computed once, when a grapheme is written (`lunatui_core::width` for scalars, plus the cluster table's cached width for clusters), and stored in bits 12–13. The diff and encoder never call Unicode tables.
- A width-2 grapheme takes its lead cell (width 2) and one continuation cell (glyph `CONTINUATION`, width 0) carrying the same style.
- Grid write APIs keep the pair atomic: overwriting either half clears the other half to a blank with the overwriter's style. A wide grapheme that doesn't fit at the right edge is replaced by a blank.
- The encoder never emits continuation cells, and advances its cursor by the lead cell's width (avoiding ratatui's #2651 class of bug).

### 4.8 Mode 2027 policy

The capability probe (PR 12, `lunatui_term::Caps`) asks DECRQM for mode 2027 (grapheme clusters). If the terminal reports the mode set or reset, lunatui enables it when it is reset, so the terminal segments clusters as lunatui does, and cluster widths follow Unicode. If the mode is unreported, unrecognized, or locked off, cluster widths come from the per-terminal width policy (§2.5, `docs/width-policy.md`). Support is probed at runtime and never assumed. `char_width` still uses the Unicode tables; `WidthPolicy::ambiguous_columns` is the ambiguous-width fallback for callers.

## 5. Invariants

These become property tests (M3) in the PRs that implement them.

1. `size_of::<Cell>() == 16`, `align_of::<Cell>() == 16`, and `Cell::from_bits(c.to_bits()) == c` for every cell.
2. Two cells are equal exactly when their `u128` words are equal. Reserved bits are zero, so equal content gives equal bits.
3. A glyph slot is a scalar value, `CONTINUATION`, or a tagged ID present in the frame's cluster table.
4. Equal clusters get equal IDs within a frame and across consecutive frames.
5. Every width-2 cell is followed by exactly one continuation cell with the same style, every continuation cell is preceded by one, and column `width − 1` is never a width-2 lead.
6. A non-zero `ExtIndex` refers to a live `ExtTable` entry. Equal rare data gets equal indices within a frame and across consecutive frames.
7. **Diff/apply:** `old + encode(diff(old, new)) == new` on the vt100 oracle (PRD §7). This is the core invariant for PRs 08–11.

## 6. Benchmarks and targets

Prototype on this Mac (Apple M5 Max, macOS 27, rustc 1.99.0, release), PR 03's dashboard frames packed into prototype cells. Changed-cell counts match ratatui's exactly (tests in `benches/tests/cell_prototype.rs`).

| criterion median | ratatui diff | packed struct | **`[u128]` rows** | PRD target |
|---|---|---|---|---|
| 200×60 identical | 64.9 µs | 7.4 µs | **3.8 µs** | ≤ 8 µs scalar, ≤ 4 µs SIMD |
| 200×60 tick (14 changed) | 64.7 µs | 7.9 µs | **3.8 µs** | |
| 200×60 scroll (3,745 changed) | 76.1 µs | 6.6 µs | **3.4 µs** | |
| 400×120 identical | 249.0 µs | 30.0 µs | **14.4 µs** | |
| 400×120 tick (25 changed) | 244.8 µs | 31.8 µs | **14.6 µs** | |
| 400×120 scroll (1,318 changed) | 254.9 µs | 30.9 µs | **12.6 µs** | |

ratatui-perf measured 4.21 µs for its `u128` row compare on the Xeon. The `[u128]` layout already meets the 8 µs scalar target, before dirty-row tracking (PR 07) skips unchanged rows entirely.

Hashing each row (FNV-1a over the four words of each cell) takes **40.1 µs** at 200×60, ten times the full compare. So dirty tracking uses per-row dirty bits and spans updated by every write (PRD §2.3), not row hashes.

Memory: one 200×60 grid is 192,000 bytes (187.5 KiB), so front and back buffers together are 375 KiB, against ratatui's 1.1 MiB.

Targets for the implementing PRs: PR 06 asserts the size and alignment. PR 07 has zero allocations for scalar writes. PR 08 diffs an unchanged 200×60 grid in ≤ 8 µs, with a gungraun baseline recorded and the CP-1 follow-up review.

## 7. Migration and compatibility

- **ratatui compatibility (Phase 6):** a ratatui `Cell` converts to a lunatui cell by interning its symbol, mapping named colors to indexed 0–15 and `Reset` to Default, and putting underline color in the `ExtTable`. The prototype `Packer` does this, minus width and ext data. Conversion back is lossless for the common subset (PR 45).
- **Lossy cases:** ratatui allows any string in a cell. lunatui stores one grapheme per cell, so a multi-grapheme ratatui symbol is split or truncated by the bridge (documented in PR 44).
- **Theme tokens:** a `Token` color must be resolved before comparing it with a concrete color, so token cells compare by token ID. A theme switch then re-encodes without re-rendering, which is the point of D5.

## 8. Tests that will prove it

| PR | Tests |
|---|---|
| 05 | Property: cluster round-trip; equal clusters get equal IDs within and across frames (invariant 4); overflow falls back to a full re-intern with correct output |
| 06 | Size and alignment asserts; `from_bits`/`to_bits` round-trip and field accessor property tests (invariants 1, 2); color and attribute encoding tables |
| 07 | Property: no sequence of writes breaks a lead/continuation pair (invariant 5); dirty spans are exact; zero allocations for scalar writes (counting allocator) |
| 08 | Property: packed diff equals a naive cell-by-cell diff; benchmark against the 8 µs target; ratatui changed-cell counts on the PR 03 scenes |
| 09–11 | Diff/apply on the vt100 oracle (invariant 7); encoder fuzzing |

## 9. Open questions for CP-1 and MB2090

1. **`CONTINUATION` value.** `0x0011_0000` sits just above U+10FFFF and needs no extra flag. The alternative is a width-0 cell with any glyph, which wastes the glyph slot. Recommendation: `0x0011_0000`.
2. **Cluster table bound.** 65,536 IDs per frame (31 bits are available). A full screen of distinct clusters at 400×120 is 48,000, so 65,536 covers any single frame. Recommendation: 65,536, with the overflow fallback.
3. **ExtTable bound and overflow.** 16-bit index, so 65,535 entries. On overflow, recommendation: drop the extra data for the overflowing cells for that frame and log it. Hyperlinks and underline colors are cosmetic, and the frame stays correct.
4. **The spare attribute bit.** Recommendation: keep it reserved (zero) until a use appears. Candidates are rapid blink (rare) and a dedicated "link" marker.
5. **Token colors in equality.** Comparing token IDs means a cell that changes from a token to the same concrete color is reported as changed. Recommendation: accept this, since it's rare and the output is still correct.
6. **Ambiguous width and mode 2027 fallbacks.** These are deferred to the per-terminal width policy (§2.5, PR 12). This ADR fixes only that width is stored per cell at write time.
