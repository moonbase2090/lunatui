// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

/// SIMD dispatch tier selected at runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DispatchLevel {
    /// Portable scalar reference (all targets).
    Scalar,
    /// x86_64 SSE2 baseline (always assumed available on x86_64).
    #[cfg(target_arch = "x86_64")]
    Sse2,
    /// x86_64 AVX2 when the CPU reports AVX2.
    #[cfg(target_arch = "x86_64")]
    Avx2,
    /// aarch64 NEON baseline.
    #[cfg(target_arch = "aarch64")]
    Neon,
}

impl DispatchLevel {
    /// Human-readable label for snapshots and logging.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Scalar => "scalar",
            #[cfg(target_arch = "x86_64")]
            Self::Sse2 => "sse2",
            #[cfg(target_arch = "x86_64")]
            Self::Avx2 => "avx2",
            #[cfg(target_arch = "aarch64")]
            Self::Neon => "neon",
        }
    }
}

#[cfg(target_arch = "x86_64")]
fn x86_avx2_available() -> bool {
    use core::arch::x86_64::{__cpuid_count, _xgetbv};
    let max_leaf = __cpuid_count(0, 0).eax;
    if max_leaf < 7 {
        return false;
    }
    let leaf1_ecx = __cpuid_count(1, 0).ecx;
    let osxsave = leaf1_ecx & (1 << 27) != 0;
    let avx = leaf1_ecx & (1 << 28) != 0;
    if !(osxsave && avx) {
        return false;
    }
    let leaf7_ebx = __cpuid_count(7, 0).ebx;
    if leaf7_ebx & (1 << 5) == 0 {
        return false;
    }
    let xcr0 = {
        // SAFETY: OSXSAVE is set, so reading XCR0 with index 0 is defined on x86_64.
        unsafe { _xgetbv(0) }
    };
    xcr0 & 0x6 == 0x6
}

/// Highest SIMD tier this process should use (cached selection happens in `simd` kernels).
#[must_use]
pub fn active_dispatch_level() -> DispatchLevel {
    #[cfg(target_arch = "x86_64")]
    {
        if x86_avx2_available() {
            return DispatchLevel::Avx2;
        }
        return DispatchLevel::Sse2;
    }
    #[cfg(target_arch = "aarch64")]
    {
        DispatchLevel::Neon
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        DispatchLevel::Scalar
    }
}

/// Every dispatch level compiled on this target (for differential tests).
#[must_use]
pub fn differential_levels() -> &'static [DispatchLevel] {
    #[cfg(target_arch = "x86_64")]
    {
        const LEVELS: [DispatchLevel; 3] = [
            DispatchLevel::Scalar,
            DispatchLevel::Sse2,
            DispatchLevel::Avx2,
        ];
        &LEVELS
    }
    #[cfg(target_arch = "aarch64")]
    {
        const LEVELS: [DispatchLevel; 2] = [DispatchLevel::Scalar, DispatchLevel::Neon];
        &LEVELS
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        const LEVELS: [DispatchLevel; 1] = [DispatchLevel::Scalar];
        &LEVELS
    }
}
