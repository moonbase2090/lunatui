// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::palette;

#[derive(Clone, Copy)]
struct Lab {
    l: f64,
    a: f64,
    b: f64,
}

/// Lowest palette index in `indices` whose `OKLab` distance to `rgb` is smallest.
pub(crate) fn nearest(rgb: [u8; 3], indices: impl IntoIterator<Item = u8>) -> u8 {
    let target = lab(rgb);
    let mut indices = indices.into_iter();
    let Some(first) = indices.next() else {
        return 0;
    };
    let mut best_index = first;
    let mut best = distance(target, lab(palette::rgb(first)));
    for index in indices {
        let gap = distance(target, lab(palette::rgb(index)));
        if gap < best {
            best = gap;
            best_index = index;
        }
    }
    best_index
}

fn distance(left: Lab, right: Lab) -> f64 {
    let dl = left.l - right.l;
    let da = left.a - right.a;
    let db = left.b - right.b;
    dl * dl + da * da + db * db
}

fn lab(rgb: [u8; 3]) -> Lab {
    let red = linear(rgb[0]);
    let green = linear(rgb[1]);
    let blue = linear(rgb[2]);
    let long = 0.412_221_470_8 * red + 0.536_332_536_3 * green + 0.051_445_992_9 * blue;
    let medium = 0.211_903_498_2 * red + 0.680_699_545_1 * green + 0.107_396_956_6 * blue;
    let short = 0.088_302_461_9 * red + 0.281_718_837_6 * green + 0.629_978_700_5 * blue;
    let long = long.cbrt();
    let medium = medium.cbrt();
    let short = short.cbrt();
    Lab {
        l: 0.210_454_255_3 * long + 0.793_617_785_0 * medium - 0.004_072_046_8 * short,
        a: 1.977_998_495_1 * long - 2.428_592_205_0 * medium + 0.450_593_709_9 * short,
        b: 0.025_904_037_1 * long + 0.782_771_766_2 * medium - 0.808_675_766_0 * short,
    }
}

fn linear(channel: u8) -> f64 {
    let value = f64::from(channel) / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}
