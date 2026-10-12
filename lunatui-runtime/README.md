# lunatui-runtime

Runtime for lunatui. `Input` reads terminal bytes on a worker thread, decodes parser events, and coalesces available bursts into frames. `Scheduler` drives idle, dirty, and animating frame timing with backpressure (PR 22).
