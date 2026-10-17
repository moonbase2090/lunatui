// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#[cfg(target_arch = "x86_64")]
pub(crate) fn first_mismatch_sse2(a: &[u128], b: &[u128]) -> Option<usize> {
    // SAFETY: dispatch selected SSE2 only when the CPU supports it.
    unsafe { first_mismatch_sse2_impl(a, b) }
}

#[cfg(target_arch = "x86_64")]
pub(crate) fn first_mismatch_avx2(a: &[u128], b: &[u128]) -> Option<usize> {
    // SAFETY: dispatch selected AVX2 only when the CPU supports it.
    unsafe { first_mismatch_avx2_impl(a, b) }
}

#[cfg(target_arch = "aarch64")]
pub(crate) fn first_mismatch_neon(a: &[u128], b: &[u128]) -> Option<usize> {
    // SAFETY: dispatch selected NEON on aarch64 targets.
    unsafe { first_mismatch_neon_impl(a, b) }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
unsafe fn first_mismatch_sse2_impl(a: &[u128], b: &[u128]) -> Option<usize> {
    use core::arch::x86_64::{__m128i, _mm_cmpeq_epi8, _mm_loadu_si128, _mm_movemask_epi8};
    let shared = a.len().min(b.len());
    for index in 0..shared {
        let mask = {
            // SAFETY: `index` is within both slice lengths.
            unsafe {
                let va = _mm_loadu_si128(a.as_ptr().add(index) as *const __m128i);
                let vb = _mm_loadu_si128(b.as_ptr().add(index) as *const __m128i);
                _mm_movemask_epi8(_mm_cmpeq_epi8(va, vb))
            }
        };
        if mask != 0xFFFF {
            return Some(index);
        }
    }
    if a.len() == b.len() {
        None
    } else {
        Some(shared)
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn first_mismatch_avx2_impl(a: &[u128], b: &[u128]) -> Option<usize> {
    use core::arch::x86_64::{
        __m128i, __m256i, _mm_cmpeq_epi8, _mm_loadu_si128, _mm_movemask_epi8, _mm256_cmpeq_epi8,
        _mm256_loadu_si256, _mm256_movemask_epi8,
    };
    let shared = a.len().min(b.len());
    let mut index = 0usize;
    while index + 2 <= shared {
        let mask = {
            // SAFETY: `index` and `index + 1` are within both slice lengths.
            unsafe {
                let va = _mm256_loadu_si256(a.as_ptr().add(index) as *const __m256i);
                let vb = _mm256_loadu_si256(b.as_ptr().add(index) as *const __m256i);
                _mm256_movemask_epi8(_mm256_cmpeq_epi8(va, vb)) as u32
            }
        };
        if mask == 0xFFFF_FFFF_u32 {
            index += 2;
            continue;
        }
        let equal_bytes = mask.trailing_ones();
        let cell_offset = usize::try_from(equal_bytes / 16).unwrap_or(usize::MAX);
        return Some(index + cell_offset);
    }
    while index < shared {
        let mask = {
            // SAFETY: `index` is within both slice lengths.
            unsafe {
                let va = _mm_loadu_si128(a.as_ptr().add(index) as *const __m128i);
                let vb = _mm_loadu_si128(b.as_ptr().add(index) as *const __m128i);
                _mm_movemask_epi8(_mm_cmpeq_epi8(va, vb))
            }
        };
        if mask != 0xFFFF {
            return Some(index);
        }
        index += 1;
    }
    if a.len() == b.len() {
        None
    } else {
        Some(shared)
    }
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn first_mismatch_neon_impl(a: &[u128], b: &[u128]) -> Option<usize> {
    use core::arch::aarch64::{uint8x16_t, vceqq_u8, vld1q_u8, vminvq_u8};
    let shared = a.len().min(b.len());
    for index in 0..shared {
        let all_equal = {
            // SAFETY: `index` is within both slice lengths.
            unsafe {
                let va = vld1q_u8(a.as_ptr().add(index).cast::<u8>());
                let vb = vld1q_u8(b.as_ptr().add(index).cast::<u8>());
                let cmp: uint8x16_t = vceqq_u8(va, vb);
                vminvq_u8(cmp) == 0xFF
            }
        };
        if !all_equal {
            return Some(index);
        }
    }
    if a.len() == b.len() {
        None
    } else {
        Some(shared)
    }
}
