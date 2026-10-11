# ADR 0002: Layout engine (DD-2)

| | |
|---|---|
| **Status** | Proposed. Approval chain (PRD M7, §T4): Muse review, then the CP-2 checkpoint review (Codex, per the seat assignments), then MB2090 sign-off |
| **Date** | 2026-10-10 |
| **Decides** | PRD §2.4 (layout engine), DD-2 |
| **Implemented by** | PR 14 (closed-form split), PR 15 (kasuari fallback and the parity suite), PR 16 (memo, `Grid`, `Stack`, `Auto`) |
| **Evidence** | `lunatui_bench::layout_prototype`. `cargo test -p lunatui-bench --test layout_prototype` and `cargo run --example layout_prototype` |

## 1. Problem

Every frame, widgets ask for rectangles inside the terminal. ratatui answers that with a [kasuari](https://crates.io/crates/kasuari) solve: a fresh linear solver, two variables per segment plus spacers, and a strength for every constraint. The PRD baseline measured **141 µs** for six mixed constraints and **11.3 ms** for twenty `Fill` constraints. The second number is most of the 16 ms frame budget before any cells are drawn.

The layout API is hard to change later. Widgets, hit testing, focus, and animation all consume the rectangles. The algorithm can change behind the API. The meanings of `Min`, `Max`, `Length`, `Percentage`, `Ratio`, `Fill`, flex, and spacing cannot, once Scorecard and the widget crate depend on them.

## 2. Decision drivers

From PRD §1.3 and §2.4:

- A layout miss of six mixed constraints is **≤ 5 µs**. Twenty `Fill` constraints are **≤ 20 µs**.
- A true conflict that falls through to kasuari, at most 20 constraints, is **≤ 1 ms** (PR 15).
- The API stays close to ratatui: the same constraint names, flex modes, spacing, and overlap.
- No `f64` on the fast path. Sizes are integers. Ties are deterministic.
- `no_std + alloc` for `lunatui-layout` (PRD D8). The fast path does not link kasuari.
- Zero allocations on a memo hit (PR 16). The miss may allocate the output `Vec`.

## 3. Options considered

| Option | Cost | Parity | Verdict |
|---|---|---|---|
| **A. kasuari on every miss**, as ratatui 0.30 does | 141 µs typical, 11.3 ms for 20 `Fill` | Exact, including the solver's tie breaks | Rejected as the primary path. It misses the 20 µs target by three orders of magnitude. |
| **B. Closed-form split for every input, kasuari only when the fast path reports a conflict** | One pass over the segments, plus a water-fill for `Fill` weights | Matches ratatui where the preferred sizes fit. Documented deltas where they do not | **Chosen.** |
| **C. Always closed-form, no solver** | Same as B, and conflicts have whatever the clip rule produces | Conflicts diverge from ratatui with no escape hatch | Rejected. `Min` floors and spacing that cannot fit need the strength tradeoff kasuari already encodes. |
| **D. Cache the kasuari solver and edit one constraint** | Incremental, still a solver | Exact | Rejected. ratatui considered this and re-solves instead. Animation changes the rect every frame, so the cache key misses anyway. A memo of the rectangles is enough (PR 16). |

## 4. Decision

### 4.1 API

`lunatui-layout` exposes the same family ratatui does, plus the three forms PR 16 adds.

- `Constraint`: `Min(u16)`, `Max(u16)`, `Length(u16)`, `Percentage(u16)`, `Ratio(u32, u32)`, `Fill(u16)`.
- `Flex`: `Legacy`, `Start` (the default, matching ratatui 0.30), `End`, `Center`, `SpaceBetween`, `SpaceEvenly`, `SpaceAround`.
- `spacing`: a positive gap between segments, or a negative overlap. Zero is the default.
- `margin`: horizontal and vertical inset, applied before the split. A margin larger than the area collapses that axis to empty.
- `split` returns one rectangle per constraint, one spacer before each segment plus one after the last, and a `needs_solver` flag.
- `Grid`, `Stack`, and `Auto` are specified in §4.7 and implemented in PR 16. They are not part of PR 14.

`Rect` stays the four-`u16` rectangle already in `lunatui-core` (`x`, `y`, `width`, `height`).

### 4.2 Closed-form split

The split runs on one axis. The length `L` is the inner width or the inner height. `n` is the number of constraints. There are `n + 1` gaps: before the first segment, between segments, and after the last.

**Reserve gaps.** For non-negative spacing `g`:

| Flex | Minimum gaps |
|---|---|
| `Legacy`, `Start`, `End`, `Center`, `SpaceBetween` | Middle gaps are `g`. End gaps are 0. |
| `SpaceEvenly` | Every gap is `g`. |
| `SpaceAround`, one segment or none | Every gap is `g`. |
| `SpaceAround`, two or more | End gaps are `g`. Middle gaps are `2g`. |

Negative spacing is an overlap on the middle gaps only. End gaps stay 0. If the reserved gaps sum to more than `L`, `needs_solver` is set, every segment is 0, and the gaps are clipped from the end until they fit.

The segment budget is `L` minus the reserved gaps.

**Preferred size.** Each constraint starts here. Percentage and ratio use largest-remainder rounding against `L` (the full axis, not the budget), which is the PRD rule.

| Constraint | Preferred size | Floor | Cap | Grows with `Fill` |
|---|---|---|---|---|
| `Length(n)` | `n` | 0 | none | no |
| `Percentage(p)` | largest remainder, §4.3 | 0 | none | no |
| `Ratio(num, den)` | largest remainder, §4.3. A zero denominator is 1 | 0 | none | no |
| `Min(n)` | `n` | `n` | none | only when flex is not `Legacy`, at weight 1 |
| `Max(n)` | `n` | 0 | `n` | no |
| `Fill(w)` | 0 | 0 | none | weight `w` |

**If the preferred sum exceeds the budget,** cut size in this order, and within an order cut the last segment first: `Fill`, `Max` (down to 0), `Ratio`, `Percentage`, `Length`, then `Min`. Cutting a `Min` below its floor sets `needs_solver`. Cutting a `Length` or a percentage does not: a pane that does not fit is clipped from the tail. That is the fast path on a small terminal.

**If the preferred sum is under the budget,** the leftover is distributed:

1. `Fill` weights, and non-`Legacy` `Min` at weight 1, by largest remainder (§4.3). A `Fill(0)` takes nothing while any positive fill weight is present. If every fill weight is 0, those fills share equally.
2. What remains depends on flex.
   - `Legacy`: give it to the last segment of the lowest-priority class that can grow. Priority, weakest first: `Ratio`, `Percentage`, `Length`, `Min`. `Max` does not grow past its cap unless nothing else can take the cells, in which case the last segment breaks its cap so the row still covers the area. This is why three `Length(20)` in 80 become 20, 20, 40, and why `Percentage(20)` plus two `Length(20)` in 80 becomes 40, 20, 20: the percentage is the weaker class.
   - `Start`: the last gap.
   - `End`: the first gap.
   - `Center`: split across the two end gaps. The last gap gets the odd cell.
   - `SpaceBetween`: split across the middle gaps, lowest index first. A single segment has no middle gap, so the segment takes the leftover.
   - `SpaceEvenly`: split across every gap, on top of the reserved minimum.
   - `SpaceAround`: end gaps get one share and middle gaps get two, on top of the reserved minimum. One segment uses the `SpaceEvenly` rule.

### 4.3 Largest remainder

For a family of percentages, the target sum is `(L * Σp + 50) / 100`. Each item starts at `floor(L * p / 100)`. The extra cells, at most one per item per pass, go to the largest remainder `L * p % 100`. Equal remainders go to the lowest index.

For ratios, each item starts at `floor(L * num / den)`. The extra cells are the rounded sum of the fractional parts (`rem / den`, half away from zero). The same tie rule applies.

`Fill` weights use the same rule: share `floor(extra * w / Σw)`, then one cell to the largest `extra * w % Σw`, lowest index on a tie.

So `Fill(1), Fill(2), Fill(3)` in 50 is 8, 17, 25. `Percentage(75)` plus `Fill(1)` in 50 is 38 and 12. Four `Ratio(1, 4)` in 50 is 13, 13, 12, 12.

### 4.4 When kasuari runs

`needs_solver` is true in two cases:

1. Reserved positive gaps do not fit in the area.
2. A `Min` floor was cut because the floors themselves exceed the budget.

PR 14 still returns the fast-path rectangles and the flag. PR 15 replaces the rectangles with a kasuari solve when the flag is set. When the flag is clear, a property test checks that the fast path and kasuari produce the same rectangles on random fitting inputs.

The fast path does not call kasuari for a ordinary overflow of `Length`, `Percentage`, or `Ratio`. Twenty `Fill` constraints never set the flag.

### 4.5 Ratatui parity

Checked against ratatui 0.30.2 in `benches/tests/layout_prototype.rs`. On the fitting corpus (preferred sizes plus reserved gaps fit, spacing ≥ 0) the rectangles match, with one recorded family of deltas:

| Case | lunatui | ratatui | Why |
|---|---|---|---|
| Four `Ratio(1, 4)` in 50 | 13, 13, 12, 12 | 13, 12, 13, 12 | Both sum to 50. lunatui gives extra cells to the lowest indexes. ratatui rounds cumulative `f64` edges, which alternates. |
| Preferred sizes exceed the area | Clip the tail, lowest priority first | kasuari equalizes equal-strength constraints (three `Length(20)` in 40 become 13, 14, 13) | The fast path stays a single clip. PR 15's suite records this. It is not a `needs_solver` case. |
| Negative spacing with `SpaceEvenly` or `SpaceAround` | Overlap applies to middle gaps only | Spacer equality pulls the end gaps too | Deferred to PR 15. |
| Four `Ratio(1, 4)` in 50, `Legacy`, spacing −1 | segments 13, 13, 12, 15 | segments 13, 12, 13, 15 | Negative overlap plus largest-remainder tie order (same family as the width-50 row). |
| Four `Ratio(1, 4)`, `Center`, spacing −1 | end gaps 1 and 2 (width 40/50/80) | end gaps 2 and 1 | Center packing with overlap keeps middle gaps only; ratatui redistributes end gaps. Recorded per width in the parity suite. |
| `Fill(0)` beside a positive `Fill` | The zero takes nothing | A weight of `1e-6` takes a dust share | A lone `Fill(0)` still takes the leftover, which matches the ratatui `Flex::Legacy` diagram. |

`Legacy` excess, `Start` / `End` / `Center` packing, `SpaceBetween` / `SpaceEvenly` / `SpaceAround` on a row that fits, and weighted `Fill` match the ratatui diagrams in its `Flex` and `Constraint` docs, including `Percentage(75)` plus `Fill(1)` at 38 and 12.

### 4.6 What PR 14 ships

`lunatui-layout` gains `constraint`, `flex`, and `solve_fast`. The public entry is `Layout::split`. No kasuari dependency. The 20 × `Fill` miss is the acceptance bench: **≤ 20 µs**. The prototype already does that split; PR 14 moves it into the crate and adds the property tests in §8.

The demo is `layout`: the same panes as `layout_prototype`, re-solved at two widths. PR 14 renames the example when the crate owns the solver.

### 4.7 PR 16: memo, `Grid`, `Stack`, `Auto`

Specified now so PR 14's rectangles do not grow a second meaning.

- **Memo.** Keyed by axis, flex, spacing, margin, the constraint list, and the area. A hit returns the stored rectangles and allocates nothing. A small LRU (64 entries; ratatui uses 500, and the fast path does not need that) lives across frames. An animated area misses and still costs microseconds.
- **`Grid`.** One closed-form solve per column track and one per row track. A cell's rectangle is the intersection. Tracks are the same `Constraint` type.
- **`Stack`.** Every child receives the full area. `z` is paint order only. No second solve.
- **`Auto`.** The child is measured against the full inner area, and that size is then a `Length`. It does not see the leftover after its siblings. If dogfood shows the leftover is the right available size, that is an ADR amendment, not a silent change in PR 16.
- **`Layout::lerp(a, b, t)`.** Interpolates each edge in integer cells. The fractional cell goes to the lowest index, same tie rule as §4.3. No solve. Used by the animation primitives (PR 25). It does not land in PR 14.

### 4.8 Memory

A `Constraint` is a copyable enum. A `Rect` is four `u16`s, 8 bytes. The fast path builds two `Vec`s (sizes and gaps) and the output rectangles. There is no solver object, no hash map of variables, and no `f64` change list. PR 16's memo stores the output rectangles, not the solver.

## 5. Invariants

These become the PR 14 property tests. Spacing below is non-negative unless a row says otherwise.

1. The sum of segment sizes and gap sizes on the axis equals the inner length.
2. Every segment lies inside the inner rectangle.
3. `Fill` sizes differ from `extra * w / Σw` by at most one cell, and the extra cells go to the largest remainders.
4. A wider area does not make the sum of segment sizes smaller.
5. Twenty `Fill(1)` on a width of `20k` or `20k + 1` never sets `needs_solver`. The first width is `k` each. The extra cell on the second width goes to index 0.
6. `needs_solver` is set when reserved gaps exceed the area, and when a `Min` floor is cut. It is clear for any mix of `Length`, `Percentage`, `Ratio`, `Max`, and `Fill` whose floors fit.
7. Empty constraints return no segments and no spacers.

## 6. Benchmarks and targets

| Miss | ratatui 0.30.2 (PRD §1.3) | Target | Prototype |
|---|---|---|---|
| 6 mixed constraints | 141 µs | ≤ 5 µs | PR 14 records the gungraun sample |
| 20 × `Fill` | 11.3 ms | ≤ 20 µs | Prototype, release, mean of 20,000 splits at width 200: **432 ns**, including the output `Vec`. PR 14 records the gungraun sample. The prototype test locks the widths |
| kasuari fallback, ≤ 20 constraints | 0.9–11 ms | ≤ 1 ms | PR 15 |

The 11.3 ms case is twenty fill variables with pairwise ratio constraints inside kasuari. The closed form replaces that with `floor(L * w / Σw)` and a remainder pass. That is why the target moves from milliseconds to microseconds without a solver.

## 7. Migration and compatibility

- **ratatui apps** keep the constraint and flex names. The bridge (PR 43) can call this split for the fitting cases and kasuari for `needs_solver`. The deltas in §4.5 are the ones a port has to know about.
- **No behavior flag for the rounding.** Lowest-index remainder is the rule. A compatibility shim that reproduces ratatui's edge rounding would reintroduce `f64` and is out of scope.
- **kasuari** is a PR 15 dependency of `lunatui-layout`, behind the fallback, not a dependency of the fast path and not a dependency of this design PR.

## 8. Tests that will prove it

| PR | Tests |
|---|---|
| DD-2 | This document's prototype: the ratatui diagrams (`Legacy` excess, weighted `Fill`, `Percentage(75)`), the 20 × `Fill` widths, `needs_solver` on a broken `Min` and on spacing that does not fit, and the parity corpus against ratatui 0.30.2 |
| 14 | Property tests for invariants 1–7. Bench: 20 × `Fill` ≤ 20 µs, six mixed constraints ≤ 5 µs. Example `layout` re-solved at two widths |
| 15 | When `needs_solver` is clear, fast path equals kasuari. When it is set, kasuari's rectangles replace the fast path. Parity suite vs ratatui, with the §4.5 rows as the allowed deltas |
| 16 | Memo hit allocates nothing. `Grid` intersections, `Stack` z-order, `Auto` measured as `Length` |

## 9. Open questions for CP-2 and MB2090

1. **Tail clip vs equalize.** When three `Length(20)` do not fit, this ADR clips from the last segment. ratatui equalizes. Recommendation: clip. It is stable, it is O(n), and a sidebar keeps its width while the last pane absorbs the loss. PR 15 does not silently switch the fast path to equalize.
2. **`Fill(0)` beside a positive weight.** Recommendation: the zero takes nothing. A lone `Fill(0)` still absorbs the leftover, matching the ratatui diagram.
3. **Remainder ties.** Recommendation: lowest index. The alternating ratatui pattern is the recorded delta in §4.5, not a bug.
4. **Memo capacity.** Recommendation: 64 stored splits, not ratatui's 500. The fast path is the cheap part.
5. **`Auto` available size.** Recommendation: measure against the full inner area, then treat the result as `Length`. Revising that to "leftover after siblings" is an ADR amendment.
6. **Overlap plus `SpaceEvenly`.** Recommendation: leave the mismatch in §4.5 for PR 15. PR 14's overlap is middle gaps only.
