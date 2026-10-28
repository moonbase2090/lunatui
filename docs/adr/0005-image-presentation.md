# ADR 0005: Collect image presentation requests during widget rendering

## Problem

`Image` currently paints a half-block fallback into `RenderCx`, while
`lunatui-term::PixelLayer` owns a separate Kitty upload cache. The interactive
runtime renders `App` directly into a `Grid` and has no way to receive image
requests. A second app pass would require callers to repeat image layout and
could place the pixels somewhere different from the fallback. The image path
also needs to choose a transport from queried capabilities and retain encoded
content across frames.

## Usage (caller's view)

Existing applications continue implementing `App::render` unchanged. An app
that wants pixel images implements the frame method and passes the frame's
image sink into `RenderCx`:

```rust,ignore
impl App for Dashboard {
    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        self.draw_cells(grid, area, clusters);
    }

    fn render_frame(&mut self, frame: &mut RenderFrame<'_>, area: Rect) {
        let (grid, clusters, images) = frame.parts();
        let mut cx = RenderCx::new(grid, clusters, &self.theme).image_sink(images);
        self.image.render(area, &mut cx);
    }
}
```

`Image::render` submits the source, stable image ID, source generation, fit,
destination rectangle, depth, and semantic opacity through the sink. If the
selected presenter accepts the request, the widget omits its cell fallback;
otherwise it paints the half-block fallback in the same rectangle.

## Shape

- `lunatui-runtime::RenderFrame` borrows the current `Grid`, `ClusterTable`,
  and backend-neutral image sink for one render pass. It does not own terminal
  protocols or widget types.
- `App::render_frame` has a default body that delegates to the existing
  `App::render`. The runtime calls `render_frame`; current applications retain
  their behavior and do not need to adopt the image path.
- `lunatui-image` owns `ImageRequest` and `ImageSink`, keeping source, fit,
  placement, and semantic opacity together without depending on widgets or a
  specific terminal protocol.
- `RenderCx` optionally borrows an `ImageSink`. `Image` submits through that
  sink after computing its inner area and before drawing the cell fallback.
- `lunatui-runtime` owns the persistent presenter and per-frame cache lifecycle.
  `lunatui-term` owns wire encoders and capability-aware selection; the
  Prismattyc client owns native negotiation and may select native output only
  after an active session is established.

The public surface is one additive frame method plus one optional sink on the
existing render context. It hides transport choice, wire encoding, image
cache, and deletion of images omitted from the next frame. It deliberately
exposes a stable ID and generation because callers that animate or replace a
source must identify when the content changes; using terminal protocol IDs or
hashing all pixels every frame would leak transport policy or spend rendering
time on content checks.

## Synthesis decision

Candidate B's same-pass frame context is the base: it preserves the existing
`App::render` contract while keeping the widget's cell and pixel placement in
one layout pass. The cross-judge recommended this shape for avoiding layout
drift. We adapted it to a default `App::render_frame` method rather than a
parallel `SceneApp`/`run_scene` API, so the ecosystem keeps one app trait and
one run loop. Candidate A's runtime-owned cache and capability selection are
retained, but its second image-layout pass is rejected. Neither candidate
provided the required protocol-accurate Sixel/iTerm2 capability queries or
text-layering policy; those remain implementation requirements, not reasons
to infer support from terminal names.

## Tradeoffs accepted

- We accept an opt-in `render_frame` override in exchange for keeping existing
  `App::render` implementations source-compatible.
- We accept explicit image IDs and generations in exchange for avoiding image
  hashing and accidental retransmission on every frame.
- We accept a terminal with no confirmed pixel protocol using half-blocks in
  exchange for never guessing support from an environment name.
- We accept deferring Prismattyc-native output until its negotiated session is
  active; advertisement alone does not grant a live transport.

## Alternatives considered

- A second `App::render_images` pass hides transport ownership, but exposes
  layout coordination to every caller and can drift from the widget's cell
  rectangle. It loses on API depth and shared geometry.
- A parallel `SceneApp` and `run_scene` path keeps the old API intact but
  duplicates app entry points and scheduler behavior. It loses on lifecycle
  coherence and creates two ways to render an application.
- Passing a `PixelLayer` directly into `RenderCx` couples widgets to one wire
  protocol and still does not give the runtime ownership of the layer. It
  loses on information hiding and state ownership.

## Open questions and risks

- What terminal pixel metrics will Sixel use to translate cell rectangles into
  pixel dimensions when no cell-pixel query succeeds?
- Which text-over-image behavior should each protocol implement when a widget
  and later text overlap the same cells?
- What is the exact terminal-reported capability exchange for iTerm2 inline
  images and Sixel support, beyond environment identification?
- Which child ticket owns Prismattyc host-side activation and lease lifetime
  after LT-80?

## Next implementation step

Add the backend-neutral request/sink types to `lunatui-image`, then thread one
optional sink through `RenderFrame`, `RenderCx`, and the existing Kitty layer
without changing the default half-block path.
