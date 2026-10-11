// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/// Which axis a [`Layout`] divides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// Columns. Segment width changes and height stays the area height.
    Horizontal,
    /// Rows. Segment height changes and width stays the area width.
    Vertical,
}

/// Where leftover space goes after constraints take their size.
///
/// This matches ratatui 0.30's `Flex` names. [`Flex::Start`] is the default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Flex {
    /// First and last gaps stay empty. Leftover cells go to the last segment that can grow.
    Legacy,
    /// Pack segments at the start. Leftover cells go to the last gap.
    #[default]
    Start,
    /// Pack segments at the end. Leftover cells go to the first gap.
    End,
    /// Split leftover cells across the first and last gaps. The last gap gets the odd cell.
    Center,
    /// Split leftover cells across the gaps between segments. End gaps stay empty.
    SpaceBetween,
    /// Split leftover cells across every gap, including both ends.
    SpaceEvenly,
    /// Like [`Flex::SpaceEvenly`] for one segment. Otherwise end gaps get half of a middle gap.
    SpaceAround,
}

/// One segment's claim on the axis being divided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Constraint {
    /// At least this many cells. In [`Flex::Legacy`] it stays at this size unless it is the
    /// leftover sink. In every other flex it also shares leftover with [`Constraint::Fill`].
    Min(u16),
    /// At most this many cells, and this many when there is room.
    Max(u16),
    /// Exactly this many cells, until the area is too small or this segment is the leftover sink.
    Length(u16),
    /// `percent` hundredths of the full axis length, rounded by largest remainder.
    Percentage(u16),
    /// `numer / denom` of the full axis length. A zero denominator is treated as one.
    Ratio(u32, u32),
    /// Share of the cells left after the other constraints. Zero weight takes nothing while any
    /// positive weight is present, and shares equally when every fill weight is zero.
    Fill(u16),
}

/// A rectangle in cell coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    /// Left column.
    pub x: u16,
    /// Top row.
    pub y: u16,
    /// Columns.
    pub width: u16,
    /// Rows.
    pub height: u16,
}

impl Rect {
    /// A rectangle at `(x, y)` with the given size.
    #[must_use]
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Segments and the gaps around them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Solved {
    /// One rectangle per constraint, in order.
    pub segments: Vec<Rect>,
    /// One gap before each segment, plus one after the last. Empty when there are no constraints.
    pub spacers: Vec<Rect>,
    /// The fast path had to break a minimum or a positive spacing. PR 15 replaces these with kasuari.
    pub needs_solver: bool,
}

/// A one-axis split. Memoization is intentionally absent: PR 16 adds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    axis: Axis,
    flex: Flex,
    spacing: i16,
    horizontal_margin: u16,
    vertical_margin: u16,
    constraints: Vec<Constraint>,
}

impl Layout {
    /// A horizontal split. Flex is [`Flex::Start`], spacing and margin are zero.
    #[must_use]
    pub fn horizontal(constraints: impl IntoIterator<Item = Constraint>) -> Self {
        Self::new(Axis::Horizontal, constraints)
    }

    /// A vertical split. Flex is [`Flex::Start`], spacing and margin are zero.
    #[must_use]
    pub fn vertical(constraints: impl IntoIterator<Item = Constraint>) -> Self {
        Self::new(Axis::Vertical, constraints)
    }

    /// Where leftover space goes.
    #[must_use]
    pub const fn flex(mut self, flex: Flex) -> Self {
        self.flex = flex;
        self
    }

    /// Cells between segments. Negative values overlap by that many cells.
    #[must_use]
    pub const fn spacing(mut self, spacing: i16) -> Self {
        self.spacing = spacing;
        self
    }

    /// Inset on the left and right, and on the top and bottom, applied before the split.
    #[must_use]
    pub const fn margin(mut self, horizontal: u16, vertical: u16) -> Self {
        self.horizontal_margin = horizontal;
        self.vertical_margin = vertical;
        self
    }

    /// Split `area` along this layout's axis.
    #[must_use]
    pub fn split(&self, area: Rect) -> Solved {
        let inner = inset(area, self.horizontal_margin, self.vertical_margin);
        let len = match self.axis {
            Axis::Horizontal => inner.width,
            Axis::Vertical => inner.height,
        };
        let (sizes, gaps, needs_solver) =
            axis_sizes(len, &self.constraints, self.spacing, self.flex);
        let (segments, spacers) = place(inner, self.axis, &sizes, &gaps);
        Solved {
            segments,
            spacers,
            needs_solver,
        }
    }

    fn new(axis: Axis, constraints: impl IntoIterator<Item = Constraint>) -> Self {
        Self {
            axis,
            flex: Flex::Start,
            spacing: 0,
            horizontal_margin: 0,
            vertical_margin: 0,
            constraints: constraints.into_iter().collect(),
        }
    }
}

struct Spec {
    lower: u64,
    prefer: u64,
    upper: u64,
    weight: u32,
    shrink: u8,
}

fn axis_sizes(
    len: u16,
    constraints: &[Constraint],
    spacing: i16,
    flex: Flex,
) -> (Vec<u16>, Vec<i64>, bool) {
    let n = constraints.len();
    if n == 0 {
        return (Vec::new(), Vec::new(), false);
    }
    let mut needs_solver = false;
    let mut gaps = minimum_gaps(n, spacing, flex);
    let middle_sum: i64 = gaps.iter().sum();
    let len_i = i64::from(len);
    if middle_sum > len_i {
        shrink_gaps(&mut gaps, middle_sum - len_i);
        return (vec![0; n], gaps, true);
    }
    let budget = u64::try_from(len_i - middle_sum).unwrap_or(0);
    let specs: Vec<Spec> = constraints.iter().map(|c| spec(*c, flex)).collect();
    let mut sizes: Vec<u64> = specs.iter().map(|s| s.prefer.min(s.upper)).collect();
    grant_family_extras(&mut sizes, constraints, len);
    for (size, spec) in sizes.iter_mut().zip(&specs) {
        *size = (*size).clamp(spec.lower.min(spec.upper), spec.upper);
    }
    let sum: u64 = sizes.iter().sum();
    if sum > budget {
        let violated = shrink(&mut sizes, &specs, sum - budget);
        needs_solver = needs_solver || violated;
    } else {
        let mut weights: Vec<u32> = specs.iter().map(|s| s.weight).collect();
        promote_zero_fills(constraints, &mut weights);
        let extra = budget - sizes.iter().sum::<u64>();
        grow_weighted(&mut sizes, &specs, &weights, extra);
        let slack = budget.saturating_sub(sizes.iter().sum());
        if flex == Flex::Legacy {
            absorb_legacy(&mut sizes, &specs, slack);
        } else {
            let left = absorb_gaps(&mut gaps, flex, i64::try_from(slack).unwrap_or(i64::MAX));
            if left > 0 {
                absorb_segments(&mut sizes, &specs, u64::try_from(left).unwrap_or(u64::MAX));
            }
        }
    }
    (sizes.into_iter().map(as_u16).collect(), gaps, needs_solver)
}

fn spec(constraint: Constraint, flex: Flex) -> Spec {
    let open = u64::from(u16::MAX);
    match constraint {
        Constraint::Min(min) => Spec {
            lower: u64::from(min),
            prefer: u64::from(min),
            upper: open,
            weight: u32::from(flex != Flex::Legacy),
            shrink: 1,
        },
        Constraint::Max(max) => Spec {
            lower: 0,
            prefer: u64::from(max),
            upper: u64::from(max),
            weight: 0,
            shrink: 5,
        },
        Constraint::Length(length) => Spec {
            lower: 0,
            prefer: u64::from(length),
            upper: open,
            weight: 0,
            shrink: 2,
        },
        Constraint::Percentage(_) | Constraint::Ratio(_, _) => Spec {
            lower: 0,
            prefer: 0,
            upper: open,
            weight: 0,
            shrink: match constraint {
                Constraint::Percentage(_) => 3,
                _ => 4,
            },
        },
        Constraint::Fill(weight) => Spec {
            lower: 0,
            prefer: 0,
            upper: open,
            weight: u32::from(weight),
            shrink: 6,
        },
    }
}

fn grant_family_extras(sizes: &mut [u64], constraints: &[Constraint], len: u16) {
    let mut percent_exact = 0_u128;
    let mut percent_floor = 0_u64;
    let mut percent_parts = Vec::new();
    let mut ratio_parts = Vec::new();
    for (index, constraint) in constraints.iter().enumerate() {
        match *constraint {
            Constraint::Percentage(percent) => {
                let numer = u128::from(len) * u128::from(percent);
                let base = u64::try_from(numer / 100).unwrap_or(u64::MAX);
                sizes[index] = base;
                percent_floor = percent_floor.saturating_add(base);
                percent_exact = percent_exact.saturating_add(numer);
                percent_parts.push((index, numer % 100, 100_u128));
            }
            Constraint::Ratio(numer, denom) => {
                let denom = u128::from(denom.max(1));
                let numer = u128::from(len) * u128::from(numer);
                let base = u64::try_from(numer / denom).unwrap_or(u64::MAX);
                sizes[index] = base;
                ratio_parts.push((index, numer % denom, denom));
            }
            _ => {}
        }
    }
    let percent_target = u64::try_from((percent_exact + 50) / 100).unwrap_or(u64::MAX);
    grant(
        sizes,
        &percent_parts,
        percent_target.saturating_sub(percent_floor),
    );
    grant(sizes, &ratio_parts, ratio_extra_budget(&ratio_parts));
}

fn grant(sizes: &mut [u64], parts: &[(usize, u128, u128)], mut extras: u64) {
    let mut spent = vec![false; sizes.len()];
    while extras > 0 {
        let mut best: Option<(usize, u128, u128)> = None;
        for &(index, rem, den) in parts {
            if spent[index] || den == 0 || rem == 0 {
                continue;
            }
            let take = match best {
                None => true,
                Some((best_index, best_rem, best_den)) => {
                    let left = rem.saturating_mul(best_den);
                    let right = best_rem.saturating_mul(den);
                    left > right || (left == right && index < best_index)
                }
            };
            if take {
                best = Some((index, rem, den));
            }
        }
        let Some((index, _, _)) = best else {
            break;
        };
        sizes[index] = sizes[index].saturating_add(1);
        spent[index] = true;
        extras -= 1;
    }
}

fn add_frac(num: u128, den: u128, rem: u128, part_den: u128) -> Option<(u128, u128)> {
    let Some(left) = num.checked_mul(part_den) else {
        return None;
    };
    let Some(right) = rem.checked_mul(den) else {
        return None;
    };
    let Some(sum) = left.checked_add(right) else {
        return None;
    };
    let Some(next_den) = den.checked_mul(part_den) else {
        return None;
    };
    let divisor = gcd(sum, next_den).max(1);
    Some((sum / divisor, next_den / divisor))
}

fn ratio_extra_budget(parts: &[(usize, u128, u128)]) -> u64 {
    let mut merged_num = 0_u128;
    let mut merged_den = 1_u128;
    let mut exact = true;
    for &(_, rem, part_den) in parts {
        if part_den == 0 {
            continue;
        }
        match add_frac(merged_num, merged_den, rem, part_den) {
            Some(pair) => (merged_num, merged_den) = pair,
            None => {
                exact = false;
                break;
            }
        }
    }
    if exact {
        round_frac(merged_num, merged_den)
    } else {
        round_sum_fractions(parts)
    }
}

fn round_sum_fractions(parts: &[(usize, u128, u128)]) -> u64 {
    let mut sum = 0.0_f64;
    for &(_, rem, part_den) in parts {
        if part_den == 0 {
            continue;
        }
        sum += (rem as f64) / (part_den as f64);
    }
    round_f64_half_away(sum)
}

fn round_f64_half_away(value: f64) -> u64 {
    if !value.is_finite() || value <= 0.0 {
        return 0;
    }
    let rounded = value.round();
    u64::try_from(rounded as u128).unwrap_or(u64::MAX)
}

fn round_frac(num: u128, den: u128) -> u64 {
    if den == 0 {
        return 0;
    }
    let half = den / 2;
    u64::try_from(num.saturating_add(half) / den).unwrap_or(u64::MAX)
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

fn shrink(sizes: &mut [u64], specs: &[Spec], mut excess: u64) -> bool {
    let mut order: Vec<usize> = (0..sizes.len()).collect();
    order.sort_by(|&left, &right| {
        specs[right]
            .shrink
            .cmp(&specs[left].shrink)
            .then(right.cmp(&left))
    });
    for &index in &order {
        if excess == 0 {
            break;
        }
        let floor = specs[index].lower;
        let cut = sizes[index].saturating_sub(floor).min(excess);
        sizes[index] -= cut;
        excess -= cut;
    }
    let violated = excess > 0;
    for &index in &order {
        if excess == 0 {
            break;
        }
        let cut = sizes[index].min(excess);
        sizes[index] -= cut;
        excess -= cut;
    }
    violated
}

fn promote_zero_fills(constraints: &[Constraint], weights: &mut [u32]) {
    let mut saw_fill = false;
    let mut saw_positive = false;
    for constraint in constraints {
        if let Constraint::Fill(weight) = *constraint {
            saw_fill = true;
            saw_positive = saw_positive || weight > 0;
        }
    }
    if saw_fill && !saw_positive {
        for (weight, constraint) in weights.iter_mut().zip(constraints) {
            if matches!(constraint, Constraint::Fill(0)) {
                *weight = 1;
            }
        }
    }
}

fn grow_weighted(sizes: &mut [u64], specs: &[Spec], weights: &[u32], extra: u64) {
    let total: u64 = weights.iter().map(|&weight| u64::from(weight)).sum();
    if total == 0 || extra == 0 {
        return;
    }
    let mut given = 0_u64;
    for (index, &weight) in weights.iter().enumerate() {
        let share = mul_div(extra, u64::from(weight), total);
        let room = specs[index].upper.saturating_sub(sizes[index]);
        let add = share.min(room);
        sizes[index] = sizes[index].saturating_add(add);
        given = given.saturating_add(add);
    }
    let mut left = extra.saturating_sub(given);
    let mut order: Vec<usize> = (0..weights.len())
        .filter(|&index| weights[index] > 0)
        .collect();
    order.sort_by(|&left_index, &right_index| {
        let left_rem = mul_rem(extra, u64::from(weights[left_index]), total);
        let right_rem = mul_rem(extra, u64::from(weights[right_index]), total);
        right_rem.cmp(&left_rem).then(left_index.cmp(&right_index))
    });
    for index in order {
        if left == 0 {
            break;
        }
        if sizes[index] >= specs[index].upper {
            continue;
        }
        sizes[index] = sizes[index].saturating_add(1);
        left -= 1;
    }
}

fn minimum_gaps(n: usize, spacing: i16, flex: Flex) -> Vec<i64> {
    let mut gaps = vec![0_i64; n + 1];
    let gap = i64::from(spacing);
    if gap < 0 {
        for slot in gaps.iter_mut().skip(1).take(n.saturating_sub(1)) {
            *slot = gap;
        }
        return gaps;
    }
    match flex {
        Flex::SpaceEvenly => gaps.fill(gap),
        Flex::SpaceAround if n <= 1 => gaps.fill(gap),
        Flex::SpaceAround => {
            gaps[0] = gap;
            if let Some(last) = gaps.last_mut() {
                *last = gap;
            }
            let last = gaps.len() - 1;
            for slot in gaps.iter_mut().take(last).skip(1) {
                *slot = gap.saturating_mul(2);
            }
        }
        Flex::Legacy | Flex::Start | Flex::End | Flex::Center | Flex::SpaceBetween => {
            for slot in gaps.iter_mut().skip(1).take(n.saturating_sub(1)) {
                *slot = gap;
            }
        }
    }
    gaps
}

fn absorb_legacy(sizes: &mut [u64], specs: &[Spec], mut slack: u64) {
    let mut ranks: Vec<u8> = specs
        .iter()
        .map(|spec| spec.shrink)
        .filter(|rank| *rank != 5)
        .collect();
    ranks.sort_unstable();
    ranks.dedup();
    ranks.reverse();
    for rank in ranks {
        for index in (0..sizes.len()).rev() {
            if slack == 0 || specs[index].shrink != rank {
                continue;
            }
            let room = specs[index].upper.saturating_sub(sizes[index]);
            let add = room.min(slack);
            sizes[index] = sizes[index].saturating_add(add);
            slack -= add;
        }
    }
    if slack > 0
        && let Some(last) = sizes.last_mut()
    {
        *last = last.saturating_add(slack);
    }
}

fn absorb_segments(sizes: &mut [u64], specs: &[Spec], mut slack: u64) {
    for index in (0..sizes.len()).rev() {
        if slack == 0 {
            break;
        }
        let room = specs[index].upper.saturating_sub(sizes[index]);
        let add = room.min(slack);
        sizes[index] = sizes[index].saturating_add(add);
        slack -= add;
    }
    if slack > 0
        && let Some(last) = sizes.last_mut()
    {
        *last = last.saturating_add(slack);
    }
}

fn absorb_gaps(gaps: &mut [i64], flex: Flex, extra: i64) -> i64 {
    if extra <= 0 || gaps.is_empty() {
        return extra.max(0);
    }
    let n = gaps.len() - 1;
    match flex {
        Flex::Legacy => extra,
        Flex::Start => {
            if let Some(last) = gaps.last_mut() {
                *last += extra;
            }
            0
        }
        Flex::End => {
            gaps[0] += extra;
            0
        }
        Flex::Center => {
            let lead = extra / 2;
            gaps[0] += lead;
            if let Some(last) = gaps.last_mut() {
                *last += extra - lead;
            }
            0
        }
        Flex::SpaceBetween => {
            let middles: Vec<usize> = (1..n).collect();
            spread(gaps, &middles, extra)
        }
        Flex::SpaceEvenly => {
            let all: Vec<usize> = (0..gaps.len()).collect();
            spread(gaps, &all, extra)
        }
        Flex::SpaceAround if n <= 1 => {
            let all: Vec<usize> = (0..gaps.len()).collect();
            spread(gaps, &all, extra)
        }
        Flex::SpaceAround => spread_around(gaps, extra),
    }
}

fn spread(gaps: &mut [i64], indexes: &[usize], extra: i64) -> i64 {
    if indexes.is_empty() {
        return extra;
    }
    let count = i64::try_from(indexes.len()).unwrap_or(1).max(1);
    let base = extra / count;
    let mut remainder = extra % count;
    for &index in indexes {
        let mut add = base;
        if remainder > 0 {
            add += 1;
            remainder -= 1;
        }
        if let Some(gap) = gaps.get_mut(index) {
            *gap += add;
        }
    }
    0
}

fn spread_around(gaps: &mut [i64], extra: i64) -> i64 {
    let n = i64::try_from(gaps.len().saturating_sub(1))
        .unwrap_or(1)
        .max(1);
    let units = n.saturating_mul(2).max(1);
    let end = extra / units;
    let mut remainder = extra % units;
    gaps[0] += end;
    if let Some(last) = gaps.last_mut() {
        *last += end;
    }
    let last = gaps.len() - 1;
    for gap in gaps.iter_mut().take(last).skip(1) {
        *gap += end.saturating_mul(2);
        if remainder > 0 {
            *gap += 1;
            remainder -= 1;
        }
    }
    if remainder > 0 {
        gaps[0] += 1;
        remainder -= 1;
    }
    if remainder > 0
        && let Some(gap) = gaps.last_mut()
    {
        *gap += 1;
    }
    0
}

fn shrink_gaps(gaps: &mut [i64], mut excess: i64) {
    for gap in gaps.iter_mut().rev() {
        if excess <= 0 {
            break;
        }
        let cut = (*gap).max(0).min(excess);
        *gap -= cut;
        excess -= cut;
    }
}

fn place(area: Rect, axis: Axis, sizes: &[u16], gaps: &[i64]) -> (Vec<Rect>, Vec<Rect>) {
    let mut cursor = 0_i64;
    let mut spacers = Vec::with_capacity(gaps.len());
    let mut segments = Vec::with_capacity(sizes.len());
    for (index, &size) in sizes.iter().enumerate() {
        let gap = gaps.get(index).copied().unwrap_or(0);
        spacers.push(along(area, axis, cursor, gap));
        cursor += gap;
        segments.push(along(area, axis, cursor, i64::from(size)));
        cursor += i64::from(size);
    }
    if let Some(&gap) = gaps.last()
        && !sizes.is_empty()
    {
        spacers.push(along(area, axis, cursor, gap));
    }
    (segments, spacers)
}

fn along(area: Rect, axis: Axis, start: i64, size: i64) -> Rect {
    let origin = u16::try_from(start.max(0)).unwrap_or(u16::MAX);
    let extent = u16::try_from(size.max(0)).unwrap_or(u16::MAX);
    match axis {
        Axis::Horizontal => Rect {
            x: area.x.saturating_add(origin),
            y: area.y,
            width: extent,
            height: area.height,
        },
        Axis::Vertical => Rect {
            x: area.x,
            y: area.y.saturating_add(origin),
            width: area.width,
            height: extent,
        },
    }
}

fn inset(area: Rect, horizontal: u16, vertical: u16) -> Rect {
    Rect {
        x: area.x.saturating_add(horizontal),
        y: area.y.saturating_add(vertical),
        width: area.width.saturating_sub(horizontal.saturating_mul(2)),
        height: area.height.saturating_sub(vertical.saturating_mul(2)),
    }
}

fn as_u16(value: u64) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

fn mul_div(value: u64, weight: u64, total: u64) -> u64 {
    if total == 0 {
        return 0;
    }
    let product = u128::from(value) * u128::from(weight);
    u64::try_from(product / u128::from(total)).unwrap_or(u64::MAX)
}

fn mul_rem(value: u64, weight: u64, total: u64) -> u64 {
    if total == 0 {
        return 0;
    }
    let product = u128::from(value) * u128::from(weight);
    u64::try_from(product % u128::from(total)).unwrap_or(0)
}

#[cfg(test)]
mod ratio_extras_tests {
    use super::{grant_family_extras, Constraint};

    #[test]
    fn large_ratio_denominators_still_allocate_extras() {
        let constraints: Vec<Constraint> = (0..8)
            .map(|index| Constraint::Ratio(1, 65_536 + index as u32))
            .collect();
        let mut sizes = vec![0_u64; constraints.len()];
        grant_family_extras(&mut sizes, &constraints, 65_535);
        let extras = sizes.iter().sum::<u64>();
        assert_eq!(extras, 8, "expected eight largest-remainder extras, got {extras}");
    }
}
