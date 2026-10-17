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

/// Highest SIMD tier this process should use (cached selection happens in `simd` kernels).
#[must_use]
pub fn active_dispatch_level() -> DispatchLevel {
    #[cfg(target_arch = "x86_64")]
    {
        if core::arch::is_x86_feature_detected!("avx2") {
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
