// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! A plain-text table of benchmark metrics.

use crate::{AllocStats, WriteStats};

/// The counts measured for one workload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Metrics {
    /// Workload name.
    pub name: &'static str,
    /// Allocations made while the workload ran.
    pub alloc: AllocStats,
    /// Writes made while the workload ran.
    pub write: WriteStats,
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "M1 red stub, used in the next commit")
)]
const HEADERS: [&str; 6] = [
    "workload",
    "allocs",
    "reallocs",
    "alloc bytes",
    "writes",
    "bytes written",
];

/// Renders `rows` as an aligned table: the name column left-aligned, the
/// counts right-aligned.
#[must_use]
pub fn table(_rows: &[Metrics]) -> String {
    String::new()
}

#[cfg(test)]
mod tests {
    use super::{HEADERS, Metrics, table};
    use crate::{AllocStats, WriteStats};

    #[test]
    fn aligns_columns_to_the_widest_cell() {
        let rows = [
            Metrics {
                name: "write_frame",
                alloc: AllocStats::default(),
                write: WriteStats {
                    writes: 1,
                    bytes: 69_632,
                    flushes: 0,
                },
            },
            Metrics {
                name: "frame",
                alloc: AllocStats {
                    allocations: 1,
                    reallocations: 0,
                    deallocations: 0,
                    bytes_allocated: 69_632,
                },
                write: WriteStats::default(),
            },
        ];
        let expected = "\
workload     allocs  reallocs  alloc bytes  writes  bytes written
write_frame       0         0            0       1          69632
frame             1         0        69632       0              0
";
        assert_eq!(table(&rows), expected);
    }

    #[test]
    fn empty_table_has_only_headers() {
        assert_eq!(table(&[]), format!("{}\n", HEADERS.join("  ")));
    }
}
