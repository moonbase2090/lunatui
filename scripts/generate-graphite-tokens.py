#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT

"""Generate the palette from a pinned commit in a local Graphite checkout."""
import argparse
from pathlib import Path
import re
import subprocess

REVISION = "337ab5d8a33bf1a968291d4a858dc89c9905df8c"
FIELDS = [
    "ground", "panel", "text", "accent", "working", "unseen", "attention",
    "accent", "hairline", "muted", "text_strong", "title_hover", "title_line",
    "hover_outline", "idle", "title_focus", "title_focus_line", "muted_focus",
    "on_attention", "cycle_head",
]
HEADER = "// Copyright (c) 2026 Moonbase 2090 LLC\n// SPDX-License-Identifier: MIT\n\n"


def rgb(hex_value):
    return [(hex_value >> 16) & 255, (hex_value >> 8) & 255, hex_value & 255]


def luminance(color):
    def linear(channel):
        sample = channel / 255
        if sample <= 0.04045:
            return sample / 12.92
        return ((sample + 0.055) / 1.055) ** 2.4
    return sum(weight * linear(channel) for weight, channel in
               zip([0.2126, 0.7152, 0.0722], color))


def accent(bar, text):
    color = [0x62, 0xA8, 0xFF]
    bar = rgb(bar)
    light = luminance(bar) > luminance(rgb(text))
    for _ in range(24):
        low, high = sorted([luminance(color), luminance(bar)])
        if (high + 0.05) / (low + 0.05) >= 3:
            break
        target = 0 if light else 255
        color = [(channel * 232 + target * 24) // 256 for channel in color]
    return color


def generate(source):
    output = HEADER + "use lunatui_core::Color;\nuse crate::Variant;\n\n"
    output += "pub(crate) fn colors(variant: Variant) -> [Color; 20] {\n    match variant {\n"
    for variant in ["Dark", "Light"]:
        block = source.split("pub const " + variant.upper() + ": Tokens = Tokens {", 1)[1].split("};", 1)[0]
        values = {key: int(value, 16) for key, value in
                  re.findall(r"(\w+): rgb\(0x([0-9a-f]+)\)", block)}
        output += f"        Variant::{variant} => [\n"
        for field in FIELDS:
            color = accent(values["bar"], values["text"]) if field == "accent" else rgb(values[field])
            output += f"            Color::Rgb({color[0]}, {color[1]}, {color[2]}),\n"
        output += "        ],\n"
    output += "    }\n}\n\n"
    output += '''#[cfg(feature = "color-profiles")]
pub(crate) fn ansi16(variant: Variant) -> [Color; 20] {
    let (ground, panel, text, strong, hover, danger_text, head) = match variant {
        Variant::Dark => (0, 0, 7, 15, 8, 0, 15),
        Variant::Light => (7, 15, 0, 0, 7, 7, 4),
    };
    [ground, panel, text, 4, 2, 3, 1, 4, 8, 8, strong, hover, 8, 4, 8, 4, 4, 8, danger_text, head].map(Color::Indexed)
}
'''
    return subprocess.run(["rustfmt", "--emit", "stdout"], input=output, text=True,
                          capture_output=True, check=True).stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("graphite", help="Path to a local Graphite repository")
    parser.add_argument("--check", action="store_true", help="Compare without writing")
    args = parser.parse_args()
    source = subprocess.check_output(["git", "-C", args.graphite, "show", REVISION + ":src/tokens.rs"], text=True)
    output = generate(source)
    target = Path(__file__).resolve().parents[1] / "lunatui-theme/src/graphite.rs"
    if args.check:
        if target.read_text() != output:
            raise SystemExit("Graphite palette differs; regenerate it")
    else:
        target.write_text(output)


if __name__ == "__main__":
    main()
