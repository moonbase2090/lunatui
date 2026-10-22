# lunatui-prismattyc

Prismattyc backend for lunatui (PRD §2.12).

## In-process grid sink (PR 52)

[`PrismattycBackend`] holds the previous frame and calls a [`HostRenderer`] with a [`FrameDiff`]: the packed [`lunatui_core::Grid`], grapheme [`lunatui_core::ClusterTable`], and [`lunatui_core::ChangedRun`] list from [`lunatui_core::diff_runs`]. No ANSI is emitted (`PresentStats::ansi_bytes` is always zero).

Prismattyc chrome can clone or patch from `frame.next` and `frame.runs` directly. Native protocol negotiation lands in later roadmap PRs.

```rust,no_run
use lunatui_core::{ClusterTable, Grid};
use lunatui_prismattyc::{HostRenderer, PrismattycBackend};

struct Chrome;

impl HostRenderer for Chrome {
    fn present_frame(&mut self, frame: &lunatui_prismattyc::FrameDiff<'_>) -> std::io::Result<()> {
        let _ = (frame.next, frame.runs);
        Ok(())
    }
}

let width = 80_u16;
let height = 24_u16;
let grid = Grid::new(width, height);
let clusters = ClusterTable::new();
let mut backend = PrismattycBackend::new(width, height, Chrome);
let _stats = match backend.present(&grid, &clusters) {
    Ok(stats) => stats,
    Err(err) => panic!("present: {err}"),
};
```
