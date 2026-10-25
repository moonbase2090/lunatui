// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

mod alloc;
/// Workloads with known costs, used to check the harness itself. They stand in for
/// real scenes until lunatui has code to measure, and the harness self-test checks
/// the counts each one documents.
pub mod calibration;
/// The frame budget (PRD §1.3): a 16 ms maximum on shared CI runners, gated on
/// every sample of every worst-case benchmark, and an 8 ms target reported on a
/// dedicated machine.
pub mod ceiling;
/// Fair lunatui vs stock ratatui dashboard end-to-end helpers.
pub mod dashboard_fair;
/// A closed-form layout proposed by ADR 0002 (DD-2). Evidence for the design, not `lunatui-layout`.
pub mod layout_prototype;
/// A prototype of the 16-byte cell proposed by ADR 0001 (DD-1), packed from ratatui buffers
/// to measure the diff it enables. It is evidence for the design, not lunatui's cell.
pub mod prototype;
/// A plain-text table of benchmark metrics.
pub mod report;
/// The ratatui baseline scenes (PRD §6.1), ported 1:1 from the measured ratatui-perf runs.
pub mod scenes;
/// Per-widget comparison of lunatui and ratatui on identical data: shared cases, frame runners and
/// the counters the parity test, the criterion bench and the report use.
pub mod widget_suite;
mod write;

pub use alloc::{AllocStats, CountingAllocator};
pub use write::{CountingWriter, WriteStats};
