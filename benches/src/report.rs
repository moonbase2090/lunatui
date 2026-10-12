// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::{array, iter};

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

impl Metrics {
    fn cells(&self) -> Row {
        [
            self.name.to_owned(),
            self.alloc.allocations.to_string(),
            self.alloc.reallocations.to_string(),
            self.alloc.bytes_allocated.to_string(),
            self.write.writes.to_string(),
            self.write.bytes.to_string(),
        ]
    }
}

const COLUMNS: usize = 6;

type Row = [String; COLUMNS];

const HEADERS: [&str; COLUMNS] = [
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
pub fn table(rows: &[Metrics]) -> String {
    let header: Row = HEADERS.map(str::to_owned);
    let body: Vec<Row> = rows.iter().map(Metrics::cells).collect();
    let widths: [usize; COLUMNS] = array::from_fn(|column| {
        iter::once(&header)
            .chain(&body)
            .map(|row| row[column].chars().count())
            .max()
            .unwrap_or(0)
    });

    iter::once(&header)
        .chain(&body)
        .map(|row| format_row(row, &widths))
        .collect()
}

fn format_row(row: &Row, widths: &[usize; COLUMNS]) -> String {
    let cells: Vec<String> = row
        .iter()
        .zip(widths)
        .enumerate()
        .map(|(column, (cell, &width))| {
            if column == 0 {
                format!("{cell:<width$}")
            } else {
                format!("{cell:>width$}")
            }
        })
        .collect();
    format!("{}\n", cells.join("  "))
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
