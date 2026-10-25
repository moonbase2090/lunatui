// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::time::{Duration, Instant};

/// Hard ceiling on the time of any single frame (PRD §1.3).
pub const FRAME_CEILING: Duration = Duration::from_millis(16);

/// Target for the time of any single frame on a dedicated machine (PRD §1.3).
pub const FRAME_TARGET: Duration = Duration::from_millis(8);

/// How a worst-case benchmark's slowest frame compares with the frame budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// The slowest frame is at or under [`FRAME_TARGET`].
    WithinTarget,
    /// The slowest frame misses [`FRAME_TARGET`] but is at or under [`FRAME_CEILING`].
    OverTarget,
    /// The slowest frame exceeds [`FRAME_CEILING`]. CI fails.
    OverCeiling,
}

/// Frame-time statistics for one worst-case benchmark.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameTimes {
    /// Number of frames timed.
    pub samples: usize,
    /// Median frame time (nearest rank).
    pub p50: Duration,
    /// 95th-percentile frame time (nearest rank).
    pub p95: Duration,
    /// 99th-percentile frame time (nearest rank), reported for context only.
    pub p99: Duration,
    /// Slowest frame. The ceiling gates on this.
    pub max: Duration,
}

impl FrameTimes {
    /// Summarizes `samples`, or returns `None` when there are none.
    #[must_use]
    pub fn from_samples(samples: &[Duration]) -> Option<Self> {
        let mut sorted = samples.to_vec();
        sorted.sort_unstable();
        Some(Self {
            samples: sorted.len(),
            p50: nearest_rank(&sorted, 50)?,
            p95: nearest_rank(&sorted, 95)?,
            p99: nearest_rank(&sorted, 99)?,
            max: *sorted.last()?,
        })
    }

    /// Returns whether every sample is at or under `ceiling`.
    #[must_use]
    pub fn within(&self, ceiling: Duration) -> bool {
        self.max <= ceiling
    }

    /// Compares the slowest frame with [`FRAME_TARGET`] and [`FRAME_CEILING`].
    #[must_use]
    pub fn verdict(&self) -> Verdict {
        if !self.within(FRAME_CEILING) {
            Verdict::OverCeiling
        } else if !self.within(FRAME_TARGET) {
            Verdict::OverTarget
        } else {
            Verdict::WithinTarget
        }
    }
}

fn nearest_rank(sorted: &[Duration], percent: usize) -> Option<Duration> {
    let rank = (percent * sorted.len()).div_ceil(100).max(1);
    sorted.get(rank - 1).copied()
}

/// Times `count` runs of `frame`, returning one sample per run.
///
/// # Errors
///
/// Returns the first error from `frame`.
pub fn sample<E>(
    count: usize,
    mut frame: impl FnMut() -> Result<(), E>,
) -> Result<Vec<Duration>, E> {
    let mut samples = Vec::with_capacity(count);
    for _ in 0..count {
        let start = Instant::now();
        frame()?;
        samples.push(start.elapsed());
    }
    Ok(samples)
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::time::Duration;

    use super::{FRAME_CEILING, FRAME_TARGET, FrameTimes, Verdict, sample};

    fn millis(range: std::ops::RangeInclusive<u64>) -> Vec<Duration> {
        range.map(Duration::from_millis).collect()
    }

    fn times_with_max(max: Duration) -> FrameTimes {
        FrameTimes {
            samples: 1,
            p50: max,
            p95: max,
            p99: max,
            max,
        }
    }

    #[test]
    fn summarizes_by_nearest_rank() {
        let expected = FrameTimes {
            samples: 100,
            p50: Duration::from_millis(50),
            p95: Duration::from_millis(95),
            p99: Duration::from_millis(99),
            max: Duration::from_millis(100),
        };
        assert_eq!(FrameTimes::from_samples(&millis(1..=100)), Some(expected));
    }

    #[test]
    fn sample_order_does_not_matter() {
        let mut samples = millis(1..=100);
        samples.reverse();
        assert_eq!(
            FrameTimes::from_samples(&samples),
            FrameTimes::from_samples(&millis(1..=100))
        );
    }

    #[test]
    fn no_samples_means_no_summary() {
        assert_eq!(FrameTimes::from_samples(&[]), None);
    }

    #[test]
    fn a_max_at_the_ceiling_passes() {
        assert!(times_with_max(FRAME_CEILING).within(FRAME_CEILING));
    }

    #[test]
    fn one_slow_frame_fails_even_when_p99_passes() {
        let mut samples = vec![Duration::from_millis(1); 999];
        samples.push(FRAME_CEILING + Duration::from_nanos(1));
        let times = FrameTimes::from_samples(&samples);
        assert_eq!(times.map(|t| t.p99), Some(Duration::from_millis(1)));
        assert_eq!(times.map(|t| t.within(FRAME_CEILING)), Some(false));
    }

    #[test]
    fn sample_times_every_run() {
        let mut runs = 0;
        let samples = sample(5, || {
            runs += 1;
            Ok::<(), Infallible>(())
        });
        assert_eq!(samples.map(|s| s.len()), Ok(5));
        assert_eq!(runs, 5);
    }

    #[test]
    fn sample_stops_at_the_first_error() {
        let mut runs = 0;
        let samples = sample(5, || {
            runs += 1;
            if runs == 2 {
                Err("frame failed")
            } else {
                Ok(())
            }
        });
        assert_eq!(samples, Err("frame failed"));
        assert_eq!(runs, 2);
    }

    #[test]
    fn the_ceiling_on_shared_ci_runners_is_16_ms_and_the_target_8_ms() {
        assert_eq!(FRAME_CEILING, Duration::from_millis(16));
        assert_eq!(FRAME_TARGET, Duration::from_millis(8));
    }

    #[test]
    fn a_max_at_the_target_is_within_target() {
        assert_eq!(
            times_with_max(FRAME_TARGET).verdict(),
            Verdict::WithinTarget
        );
    }

    #[test]
    fn a_max_just_over_the_target_is_over_target() {
        let max = FRAME_TARGET + Duration::from_nanos(1);
        assert_eq!(times_with_max(max).verdict(), Verdict::OverTarget);
    }

    #[test]
    fn a_max_at_the_ceiling_is_over_target_only() {
        assert_eq!(times_with_max(FRAME_CEILING).verdict(), Verdict::OverTarget);
    }

    #[test]
    fn a_max_just_over_the_ceiling_is_over_ceiling() {
        let max = FRAME_CEILING + Duration::from_nanos(1);
        assert_eq!(times_with_max(max).verdict(), Verdict::OverCeiling);
    }
}
