# lunatui-runtime

Runtime for lunatui. `Input` reads terminal bytes on a worker thread, decodes parser events, and coalesces available bursts into frames. `Scheduler` drives idle, dirty, and animating frame timing with backpressure (PR 22). `Clock`, `Ease`, `Spring`, and `Tween` provide frame-rate-independent animation primitives (PR 25). With the `app` feature, `App` and `run` wire input, scheduler, grid diff, and session present for interactive examples (PR 23). With the `app` feature, `HitMap` and `FocusManager` support pointer hit-testing and keyboard focus (PR 24).
