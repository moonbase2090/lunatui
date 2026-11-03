// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

const MAX_STRING: usize = 4096;
#[derive(Default)]
enum Mode {
    #[default]
    Plain,
    Escape,
    String {
        osc: bool,
        escaped: bool,
        discard: bool,
    },
}
#[derive(Default)]
pub(super) struct OutputFilter {
    mode: Mode,
    pending: Vec<u8>,
}
impl OutputFilter {
    pub(super) fn feed<CB: vt100::Callbacks>(
        &mut self,
        parser: &mut vt100::Parser<CB>,
        bytes: &[u8],
    ) {
        for &byte in bytes {
            match self.mode {
                Mode::Plain => {
                    if byte == 0x1b {
                        self.mode = Mode::Escape;
                    } else {
                        parser.process(std::slice::from_ref(&byte));
                    }
                }
                Mode::Escape => {
                    if matches!(byte, b']' | b'P' | b'X' | b'^' | b'_') {
                        parser.process(&[0x18]);
                        self.pending.clear();
                        self.pending.extend_from_slice(&[0x1b, byte]);
                        self.mode = Mode::String {
                            osc: byte == b']',
                            escaped: false,
                            discard: false,
                        };
                    } else if byte != 0x1b {
                        parser.process(&[0x1b, byte]);
                        self.mode = Mode::Plain;
                    }
                }
                Mode::String {
                    osc,
                    escaped,
                    mut discard,
                } => {
                    if matches!(byte, 0x18 | 0x1a) {
                        self.pending.clear();
                        self.mode = Mode::Plain;
                        continue;
                    }
                    if self.pending.len() >= MAX_STRING {
                        self.pending.clear();
                        discard = true;
                    }
                    if !discard {
                        self.pending.push(byte);
                    }
                    if (osc && byte == 7) || (escaped && byte == b'\\') {
                        if !discard {
                            parser.process(&self.pending);
                        }
                        self.pending.clear();
                        self.mode = Mode::Plain;
                    } else {
                        self.mode = Mode::String {
                            osc,
                            escaped: byte == 0x1b,
                            discard,
                        };
                    }
                }
            }
        }
    }
}
