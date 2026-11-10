// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#[inline]
fn byte_is_printable(byte: u8) -> bool {
    (b' '..=b'~').contains(&byte)
}

/// Scalar reference: count printable ASCII columns (`0x20`..=`0x7e`).
#[must_use]
pub fn ascii_display_width_scalar(text: &str) -> usize {
    debug_assert!(text.is_ascii());
    text.bytes().filter(|byte| byte_is_printable(*byte)).count()
}

#[cfg(target_arch = "x86_64")]
pub(crate) fn ascii_display_width_sse2(text: &str) -> usize {
    // SAFETY: `x86_64` always provides SSE2, and the dispatcher only selects this kernel there.
    unsafe { ascii_display_width_sse2_impl(text) }
}

#[cfg(target_arch = "x86_64")]
pub(crate) fn ascii_display_width_avx2(text: &str) -> usize {
    // SAFETY: the dispatcher selects AVX2 only when the CPU and OS report it. Those CPUs have POPCNT.
    unsafe { ascii_display_width_avx2_impl(text) }
}

#[cfg(target_arch = "aarch64")]
pub(crate) fn ascii_display_width_neon(text: &str) -> usize {
    // SAFETY: `aarch64` always provides NEON, and the dispatcher only selects this kernel there.
    unsafe { ascii_display_width_neon_impl(text) }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
unsafe fn ascii_display_width_sse2_impl(text: &str) -> usize {
    use core::arch::x86_64::{_mm_and_si128, _mm_cmpgt_epi8, _mm_movemask_epi8, _mm_set1_epi8};

    let bytes = text.as_bytes();
    let mut index = 0;
    let mut width = 0;
    // SAFETY: SSE2 load stays inside `bytes`; `index + 16 <= len` before each load.
    unsafe {
        let lo_bound = _mm_set1_epi8(0x1f);
        let hi_bound = _mm_set1_epi8(0x7f);
        let rest = bytes.len() & !15;
        let ptr = bytes.as_ptr();
        while index < rest {
            let chunk = ptr
                .add(index)
                .cast::<core::arch::x86_64::__m128i>()
                .read_unaligned();
            let above_space = _mm_cmpgt_epi8(chunk, lo_bound);
            let below_del = _mm_cmpgt_epi8(hi_bound, chunk);
            let printable = _mm_and_si128(above_space, below_del);
            width += mask_ones(_mm_movemask_epi8(printable));
            index += 16;
        }
    }
    width + scalar_tail(bytes, index)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2,popcnt")]
unsafe fn ascii_display_width_avx2_impl(text: &str) -> usize {
    use core::arch::x86_64::{
        _mm_and_si128, _mm_cmpgt_epi8, _mm_movemask_epi8, _mm_set1_epi8, _mm256_and_si256,
        _mm256_cmpgt_epi8, _mm256_movemask_epi8, _mm256_set1_epi8,
    };

    let bytes = text.as_bytes();
    let mut index = 0;
    let mut width = 0;
    // SAFETY: AVX2 and SSE2 loads stay inside `bytes`; each load checks the remaining length.
    unsafe {
        let lo_bound = _mm256_set1_epi8(0x1f);
        let hi_bound = _mm256_set1_epi8(0x7f);
        let rest = bytes.len() & !31;
        let ptr = bytes.as_ptr();
        while index < rest {
            let chunk = ptr
                .add(index)
                .cast::<core::arch::x86_64::__m256i>()
                .read_unaligned();
            let above_space = _mm256_cmpgt_epi8(chunk, lo_bound);
            let below_del = _mm256_cmpgt_epi8(hi_bound, chunk);
            let printable = _mm256_and_si256(above_space, below_del);
            width += mask_ones(_mm256_movemask_epi8(printable));
            index += 32;
        }
        if index + 16 <= bytes.len() {
            let chunk = ptr
                .add(index)
                .cast::<core::arch::x86_64::__m128i>()
                .read_unaligned();
            let above_space = _mm_cmpgt_epi8(chunk, _mm_set1_epi8(0x1f));
            let below_del = _mm_cmpgt_epi8(_mm_set1_epi8(0x7f), chunk);
            let printable = _mm_and_si128(above_space, below_del);
            width += mask_ones(_mm_movemask_epi8(printable));
            index += 16;
        }
    }
    width + scalar_tail(bytes, index)
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn ascii_display_width_neon_impl(text: &str) -> usize {
    use core::arch::aarch64::{vaddlvq_u8, vandq_u8, vcgtq_u8, vdupq_n_u8, vld1q_u8};

    let bytes = text.as_bytes();
    let mut index = 0;
    let mut width = 0;
    // SAFETY: NEON load stays inside `bytes`; `index + 16 <= len` before each load.
    unsafe {
        let lo_bound = vdupq_n_u8(0x1f);
        let hi_bound = vdupq_n_u8(0x7f);
        let rest = bytes.len() & !15;
        let ptr = bytes.as_ptr();
        while index < rest {
            let chunk = vld1q_u8(ptr.add(index));
            let above_space = vcgtq_u8(chunk, lo_bound);
            let below_del = vcgtq_u8(hi_bound, chunk);
            let printable = vandq_u8(above_space, below_del);
            width += usize::from(vaddlvq_u8(printable)) / 255;
            index += 16;
        }
    }
    width + scalar_tail(bytes, index)
}

#[inline]
fn mask_ones(mask: i32) -> usize {
    let ones = u16::try_from(mask.count_ones()).unwrap_or(u16::MAX);
    usize::from(ones)
}

fn scalar_tail(bytes: &[u8], mut index: usize) -> usize {
    let mut width = 0;
    while index < bytes.len() {
        if byte_is_printable(bytes[index]) {
            width += 1;
        }
        index += 1;
    }
    width
}
