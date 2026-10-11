// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

const ANSI: [[u8; 3]; 16] = [
    [0, 0, 0],
    [205, 0, 0],
    [0, 205, 0],
    [205, 205, 0],
    [0, 0, 238],
    [205, 0, 205],
    [0, 205, 205],
    [229, 229, 229],
    [127, 127, 127],
    [255, 0, 0],
    [0, 255, 0],
    [255, 255, 0],
    [92, 92, 255],
    [255, 0, 255],
    [0, 255, 255],
    [255, 255, 255],
];

const CUBE: [u8; 6] = [0, 95, 135, 175, 215, 255];

pub(crate) fn rgb(index: u8) -> [u8; 3] {
    if index < 16 {
        return ANSI[usize::from(index)];
    }
    if index < 232 {
        let offset = usize::from(index - 16);
        return [CUBE[offset / 36], CUBE[(offset / 6) % 6], CUBE[offset % 6]];
    }
    let gray = 8 + 10 * (index - 232);
    [gray, gray, gray]
}
