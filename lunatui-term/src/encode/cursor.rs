// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

pub(crate) fn write_cup(buffer: &mut Vec<u8>, col: u16, row: u16) {
    buffer.extend_from_slice(b"\x1b[");
    push_u16(buffer, row + 1);
    buffer.push(b';');
    push_u16(buffer, col + 1);
    buffer.push(b'H');
}

fn push_u16(buffer: &mut Vec<u8>, value: u16) {
    let mut digits = [0u8; 5];
    let mut len = 0;
    let mut rest = value;
    if rest == 0 {
        buffer.push(b'0');
        return;
    }
    while rest > 0 {
        digits[len] = b'0' + (rest % 10) as u8;
        len += 1;
        rest /= 10;
    }
    while len > 0 {
        len -= 1;
        buffer.push(digits[len]);
    }
}
