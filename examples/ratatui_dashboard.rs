// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("README.md")]

use std::io;
use std::thread;
use std::time::{Duration, Instant};

use lunatui_bench::ceiling::{FRAME_TARGET, FrameTimes, Verdict};
use lunatui_bench::scenes::ratatui::dashboard::{self, Model};
use ratatui::DefaultTerminal;

const FRAMES: u32 = 240;
const FRAME_INTERVAL: Duration = Duration::from_micros(16_667);

fn main() -> io::Result<()> {
    let mut terminal = ratatui::try_init()?;
    let drawn = draw_frames(&mut terminal);
    ratatui::try_restore()?;
    let size = terminal.size()?;
    let times = FrameTimes::from_samples(&drawn?)
        .ok_or_else(|| io::Error::other("no frames were drawn"))?;
    println!("{}", summary(size.width, size.height, times));
    Ok(())
}

fn draw_frames(terminal: &mut DefaultTerminal) -> io::Result<Vec<Duration>> {
    let mut model = Model::new();
    let mut draw_times = Vec::new();
    for tick in 0..FRAMES {
        model.tick = tick;
        model.scroll = usize::try_from(tick / 4 % 300).unwrap_or(0);
        let start = Instant::now();
        terminal.draw(|frame| dashboard::render(&model, frame.area(), frame.buffer_mut()))?;
        let elapsed = start.elapsed();
        draw_times.push(elapsed);
        thread::sleep(FRAME_INTERVAL.saturating_sub(elapsed));
    }
    Ok(draw_times)
}

fn summary(width: u16, height: u16, times: FrameTimes) -> String {
    let verdict = match times.verdict() {
        Verdict::WithinTarget => "within",
        Verdict::OverTarget | Verdict::OverCeiling => "over",
    };
    format!(
        "ratatui 0.30.2 dashboard, {width}x{height}, {} frames: p50 {}, p99 {}, max {} ({verdict} the {} target)",
        times.samples,
        millis(times.p50),
        millis(times.p99),
        millis(times.max),
        millis(FRAME_TARGET),
    )
}

fn millis(duration: Duration) -> String {
    format!("{:.3} ms", duration.as_secs_f64() * 1000.0)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use lunatui_bench::ceiling::FrameTimes;

    use super::summary;

    #[test]
    fn summary_reports_size_percentiles_and_target() {
        let times = FrameTimes {
            samples: 240,
            p50: Duration::from_micros(500),
            p99: Duration::from_micros(1_250),
            max: Duration::from_millis(2),
        };
        assert_eq!(
            summary(200, 60, times),
            "ratatui 0.30.2 dashboard, 200x60, 240 frames: p50 0.500 ms, p99 1.250 ms, max 2.000 ms (within the 8.000 ms target)"
        );
    }

    #[test]
    fn summary_flags_a_missed_target() {
        let times = FrameTimes {
            samples: 1,
            p50: Duration::from_millis(9),
            p99: Duration::from_millis(9),
            max: Duration::from_millis(9),
        };
        assert!(summary(400, 120, times).ends_with("(over the 8.000 ms target)"));
    }
}
