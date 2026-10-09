// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

extern crate alloc;

use alloc::vec::Vec;

use crate::grid::Grid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChangedRun {
    pub row: u16,
    pub col: u16,
    pub width: u16,
}

#[must_use]
pub fn diff_runs(_previous: &Grid, _next: &Grid) -> Vec<ChangedRun> {
    Vec::new()
}

#[must_use]
pub fn diff_runs_naive(_previous: &Grid, _next: &Grid) -> Vec<ChangedRun> {
    Vec::new()
}
