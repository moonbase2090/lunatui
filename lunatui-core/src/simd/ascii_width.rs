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
    // SAFETY: x86_64 always has SSE2, and dispatch selects this kernel only there.
    unsafe { ascii_display_width_sse2_impl(text) }
}

#[cfg(target_arch = "x86_64")]
pub(crate) fn ascii_display_width_avx2(text: &str) -> usize {
    // SAFETY: dispatch selects AVX2 only when the CPU and OS report it.
    unsafe { ascii_display_width_avx2_impl(text) }
}

#[cfg(target_arch = "aarch64")]
pub(crate) fn ascii_display_width_neon(text: &str) -> usize {
    ascii_display_width_neon_impl(text)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
unsafe fn ascii_display_width_sse2_impl(text: &str) -> usize {
    use core::arch::x86_64::{
        _mm_add_epi64, _mm_and_si128, _mm_cmpgt_epi8, _mm_cvtsi128_si64, _mm_loadu_si128,
        _mm_movemask_epi8, _mm_sad_epu8, _mm_set1_epi8, _mm_setzero_si128, _mm_unpackhi_epi64,
    };

    let bytes = text.as_bytes();
    let mut index = 0;
    let mut width = 0usize;
    // SAFETY: SSE2 is enabled on this function; each load stays inside `bytes`.
    unsafe {
        let lo_bound = _mm_set1_epi8(0x1f);
        let hi_bound = _mm_set1_epi8(0x7f);
        let zero = _mm_setzero_si128();
        let mut acc = zero;
        while index + 64 <= bytes.len() {
            let base = bytes.as_ptr().add(index);
            for offset in [0, 16, 32, 48] {
                let chunk = _mm_loadu_si128(base.add(offset).cast());
                let printable = _mm_and_si128(
                    _mm_cmpgt_epi8(chunk, lo_bound),
                    _mm_cmpgt_epi8(hi_bound, chunk),
                );
                acc = _mm_add_epi64(acc, _mm_sad_epu8(printable, zero));
            }
            index += 64;
        }
        let folded = _mm_add_epi64(acc, _mm_unpackhi_epi64(acc, acc));
        width += usize::try_from(_mm_cvtsi128_si64(folded)).unwrap_or(0) / 255;
        while index + 16 <= bytes.len() {
            let chunk = _mm_loadu_si128(bytes.as_ptr().add(index).cast());
            let printable = _mm_and_si128(
                _mm_cmpgt_epi8(chunk, lo_bound),
                _mm_cmpgt_epi8(hi_bound, chunk),
            );
            let mask = _mm_movemask_epi8(printable);
            width += if mask == 0xFFFF {
                16
            } else {
                mask.count_ones() as usize
            };
            index += 16;
        }
    }
    while index < bytes.len() {
        if byte_is_printable(bytes[index]) {
            width += 1;
        }
        index += 1;
    }
    width
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn ascii_display_width_avx2_impl(text: &str) -> usize {
    use core::arch::x86_64::{
        _mm_add_epi64, _mm_cvtsi128_si64, _mm_unpackhi_epi64, _mm256_add_epi64, _mm256_and_si256,
        _mm256_castsi256_si128, _mm256_cmpgt_epi8, _mm256_extracti128_si256, _mm256_loadu_si256,
        _mm256_movemask_epi8, _mm256_sad_epu8, _mm256_set1_epi8, _mm256_setzero_si256,
    };

    let bytes = text.as_bytes();
    let mut index = 0;
    let mut width = 0usize;
    // SAFETY: AVX2 is enabled on this function; each load stays inside `bytes`.
    unsafe {
        let lo_bound = _mm256_set1_epi8(0x1f);
        let hi_bound = _mm256_set1_epi8(0x7f);
        let zero = _mm256_setzero_si256();
        let mut acc = zero;
        while index + 128 <= bytes.len() {
            let base = bytes.as_ptr().add(index);
            for offset in [0, 32, 64, 96] {
                let chunk = _mm256_loadu_si256(base.add(offset).cast());
                let printable = _mm256_and_si256(
                    _mm256_cmpgt_epi8(chunk, lo_bound),
                    _mm256_cmpgt_epi8(hi_bound, chunk),
                );
                acc = _mm256_add_epi64(acc, _mm256_sad_epu8(printable, zero));
            }
            index += 128;
        }
        let low = _mm256_castsi256_si128(acc);
        let high = _mm256_extracti128_si256(acc, 1);
        let folded = _mm_add_epi64(low, high);
        let folded = _mm_add_epi64(folded, _mm_unpackhi_epi64(folded, folded));
        width += usize::try_from(_mm_cvtsi128_si64(folded)).unwrap_or(0) / 255;
        while index + 32 <= bytes.len() {
            let chunk = _mm256_loadu_si256(bytes.as_ptr().add(index).cast());
            let printable = _mm256_and_si256(
                _mm256_cmpgt_epi8(chunk, lo_bound),
                _mm256_cmpgt_epi8(hi_bound, chunk),
            );
            let mask = _mm256_movemask_epi8(printable);
            width += if mask == -1 {
                32
            } else {
                mask.count_ones() as usize
            };
            index += 32;
        }
    }
    while index < bytes.len() {
        if byte_is_printable(bytes[index]) {
            width += 1;
        }
        index += 1;
    }
    width
}

#[cfg(target_arch = "aarch64")]
fn ascii_display_width_neon_impl(text: &str) -> usize {
    use core::arch::aarch64::{vaddlvq_u8, vandq_u8, vcgtq_u8, vdupq_n_u8, vld1q_u8};

    let bytes = text.as_bytes();
    let mut index = 0;
    let mut width = 0;
    // SAFETY: NEON intrinsics; `index` stays within `bytes`.
    unsafe {
        let lo_bound = vdupq_n_u8(0x1f);
        let hi_bound = vdupq_n_u8(0x7f);

        while index + 16 <= bytes.len() {
            let chunk = vld1q_u8(bytes.as_ptr().add(index));
            let above_space = vcgtq_u8(chunk, lo_bound);
            let below_del = vcgtq_u8(hi_bound, chunk);
            let printable = vandq_u8(above_space, below_del);
            width += vaddlvq_u8(printable) as usize / 255;
            index += 16;
        }
    }
    while index < bytes.len() {
        if byte_is_printable(bytes[index]) {
            width += 1;
        }
        index += 1;
    }
    width
}
