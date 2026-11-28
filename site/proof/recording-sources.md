# Gallery recording sources

The gallery uses recordings from the [v0.1.44 release](https://github.com/moonbase2090/lunatui/tree/v0.1.44). The animations and still frames in `site/assets/media/modes/` are served with the site.

| Gallery entry | Source at the release tag | Preparation |
| --- | --- | --- |
| Form, LogView, Monthly, Accordion, BigText | `docs/media/{form,logs,calendar,accordion,bigtext}.{webp,png}` | The original animations and scaled stills. These widgets draw cells in both modes. |
| ProgressBar, BarChart | `docs/media/{progress-bar,barchart}.webp` | Crop the two panes from the same terminal recording. The left pane draws cells. The right pane demonstrates the widget’s native pixel option. Use its first four frames, before the example toggles it back to cells. |
| Sparkline | `docs/media/sparkline.webp` and `docs/media/graphite-activity-monitor.gif` | Crop the left terminal pane for Cell. For Graphite, take the activity monitor’s detail trace at (800, 192), size 736 × 96 pixels. |
| Activity monitor | `examples/activity_monitor.rs` and `docs/media/graphite-activity-monitor.gif` | Run the release example with an output directory. Render its ten-frame `cells.cast` with `agg --font-size 20 --last-frame-duration 1`. The Graphite animation comes from the release recording of the same scene. |

The mode switch swaps the recording and its caption. The Play recordings control switches between the animation and a still. Visitors who request reduced motion start with still frames.

Each recording's alt text describes its opening frame in words: the visible text, counts and values, so a screen reader or a search can read what the frame shows. The activity monitor frame is 960 pixels wide. On a narrow screen it keeps that width inside a sideways scroller so its text stays readable.
