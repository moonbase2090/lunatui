// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

extern crate alloc;

use alloc::vec::Vec;

use lunatui_core::Rect;

use crate::constraint::Constraint;
use crate::flex::{Axis, Flex};

struct Spec {
    lower: u64,
    prefer: u64,
    upper: u64,
    weight: u32,
    shrink: u8,
}

/// Closed-form segment sizes and gaps for one axis (ADR 0002 §4.2).
pub(crate) fn axis_sizes(
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
        return (alloc::vec![0; n], gaps, true);
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
    let mut spent = alloc::vec![false; sizes.len()];
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
    let left = num.checked_mul(part_den)?;
    let right = rem.checked_mul(den)?;
    let sum = left.checked_add(right)?;
    let next_den = den.checked_mul(part_den)?;
    let divisor = gcd(sum, next_den).max(1);
    Some((sum / divisor, next_den / divisor))
}

fn ratio_extra_budget(parts: &[(usize, u128, u128)]) -> u64 {
    if let Some((num, den)) = merge_fractions_u128(parts) {
        return round_frac(num, den);
    }
    let (num, den) = merge_fractions_big(parts);
    round_frac_big(&num, &den)
}

fn merge_fractions_u128(parts: &[(usize, u128, u128)]) -> Option<(u128, u128)> {
    let mut num = 0_u128;
    let mut den = 1_u128;
    for &(_, rem, part_den) in parts {
        if part_den == 0 {
            continue;
        }
        (num, den) = add_frac(num, den, rem, part_den)?;
    }
    Some((num, den))
}

fn merge_fractions_big(
    parts: &[(usize, u128, u128)],
) -> (num_bigint::BigUint, num_bigint::BigUint) {
    use num_bigint::BigUint;
    use num_traits::{One, Zero};

    let mut num = BigUint::zero();
    let mut den = BigUint::one();
    for &(_, rem, part_den) in parts {
        if part_den == 0 {
            continue;
        }
        let rem_b = BigUint::from(rem);
        let part_b = BigUint::from(part_den);
        num = num * &part_b + rem_b * &den;
        den *= part_b;
        let gcd = bigint_gcd(&num, &den);
        if gcd > BigUint::one() {
            num /= &gcd;
            den /= &gcd;
        }
    }
    (num, den)
}

fn bigint_gcd(left: &num_bigint::BigUint, right: &num_bigint::BigUint) -> num_bigint::BigUint {
    use num_traits::Zero;

    let mut a = left.clone();
    let mut b = right.clone();
    if b.is_zero() {
        return a;
    }
    while !b.is_zero() {
        let rem = a % &b;
        a = b;
        b = rem;
    }
    a
}

fn round_frac_big(num: &num_bigint::BigUint, den: &num_bigint::BigUint) -> u64 {
    use num_bigint::BigUint;
    use num_traits::{ToPrimitive, Zero};

    if den.is_zero() {
        return 0;
    }
    let rounded = (num * BigUint::from(2_u32) + den) / (den * BigUint::from(2_u32));
    rounded.to_u64().unwrap_or(u64::MAX)
}

fn round_frac(num: u128, den: u128) -> u64 {
    if den == 0 {
        return 0;
    }
    let quotient = num / den;
    let remainder = num % den;
    if remainder == 0 {
        return u64::try_from(quotient).unwrap_or(u64::MAX);
    }
    let half = den / 2;
    let round_up = match remainder.checked_add(half) {
        Some(sum) => sum >= den,
        None => true,
    };
    let rounded = quotient + u128::from(round_up);
    u64::try_from(rounded).unwrap_or(u64::MAX)
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
    let mut gaps = alloc::vec![0_i64; n + 1];
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

pub(crate) fn place(area: Rect, axis: Axis, sizes: &[u16], gaps: &[i64]) -> (Vec<Rect>, Vec<Rect>) {
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
    let axis_len = match axis {
        Axis::Horizontal => i64::from(area.width),
        Axis::Vertical => i64::from(area.height),
    };
    let size = size.max(0);
    let lo = start.max(0).min(axis_len);
    let hi = (start + size).min(axis_len);
    let extent = (hi - lo).max(0);
    let origin = u16::try_from(lo).unwrap_or(u16::MAX);
    let extent = u16::try_from(extent).unwrap_or(u16::MAX);
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

pub(crate) fn inset(area: Rect, horizontal: u16, vertical: u16) -> Rect {
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
    use alloc::vec::Vec;

    use super::{Constraint, Flex, axis_sizes, grant_family_extras};

    #[test]
    fn large_ratio_denominators_still_allocate_extras() {
        let constraints: Vec<Constraint> = (0..8_u32)
            .map(|denom| Constraint::Ratio(1, 65_536 + denom))
            .collect();
        let mut sizes = alloc::vec![0_u64; constraints.len()];
        grant_family_extras(&mut sizes, &constraints, 65_535);
        let extras = sizes.iter().sum::<u64>();
        assert_eq!(
            extras, 8,
            "expected eight largest-remainder extras, got {extras}"
        );
    }

    #[test]
    fn merged_fraction_half_add_overflow_still_rounds_extras() {
        let constraints = [
            Constraint::Ratio(33_227, 3_350_000_003),
            Constraint::Ratio(33_228, 3_350_001_007),
            Constraint::Ratio(33_229, 3_350_002_003),
            Constraint::Ratio(33_230, 3_350_003_003),
        ];
        let mut sizes = alloc::vec![0_u64; constraints.len()];
        grant_family_extras(&mut sizes, &constraints, 65_535);
        assert_eq!(
            sizes.iter().sum::<u64>(),
            3,
            "fractional sum is about 2.6, so half-away rounding grants three extras"
        );
    }

    #[test]
    fn near_half_ratio_extras_stay_zero_on_a_unit_axis() {
        let constraints = [
            Constraint::Ratio(200_000_000, 2_000_000_001),
            Constraint::Ratio(200_000_000, 2_000_000_003),
            Constraint::Ratio(166_728_816, 2_000_000_021),
            Constraint::Ratio(114_745_757, 2_000_000_023),
            Constraint::Ratio(318_525_437, 2_000_000_041),
        ];
        let mut sizes = alloc::vec![0_u64; constraints.len()];
        grant_family_extras(&mut sizes, &constraints, 1);
        assert_eq!(
            sizes.iter().sum::<u64>(),
            0,
            "fractional sum is just under one half, so no extras"
        );
    }

    #[test]
    fn twenty_fills_never_set_needs_solver() {
        let constraints = alloc::vec![Constraint::Fill(1); 20];
        let (sizes, _, needs_solver) = axis_sizes(200, &constraints, 0, Flex::Start);
        assert_eq!(sizes, alloc::vec![10; 20]);
        assert!(!needs_solver);
    }
}
