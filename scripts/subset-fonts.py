#!/usr/bin/env python3
# Writes the compact font files in lunatui-fonts/fonts/compact from the full files beside them.
#
# Each compact file keeps only the characters in RANGES, which must match
# lunatui_fonts::COMPACT_RANGES (the lunatui-fonts tests check every compact file against it).
# Hinting and layout tables are dropped, because Graphite text is drawn without them.
#
# Needs fontTools (pip install fonttools==4.68.0). Usage: subset-fonts.py
import pathlib
import sys

from fontTools import subset

RANGES = [
    (0x0020, 0x007E),  # Basic Latin
    (0x00A0, 0x017F),  # Latin-1 Supplement, Latin Extended-A
    (0x2000, 0x206F),  # General Punctuation
    (0x20A0, 0x20CF),  # Currency Symbols
    (0x2100, 0x214F),  # Letterlike Symbols
    (0x2190, 0x21FF),  # Arrows
    (0x2200, 0x22FF),  # Mathematical Operators
    (0x2300, 0x23FF),  # Miscellaneous Technical
    (0x2500, 0x259F),  # Box Drawing, Block Elements
    (0x25A0, 0x25FF),  # Geometric Shapes
    (0x2713, 0x2718),  # Check marks and crosses
    (0xFFFD, 0xFFFD),  # Replacement character
]

FILES = [
    "Inter-Regular.ttf",
    "Inter-Bold.ttf",
    "JetBrainsMono-Regular.ttf",
    "JetBrainsMono-Bold.ttf",
]


def main():
    fonts = pathlib.Path(__file__).resolve().parent.parent / "lunatui-fonts" / "fonts"
    (fonts / "compact").mkdir(exist_ok=True)
    unicodes = [code for first, last in RANGES for code in range(first, last + 1)]
    for name in FILES:
        options = subset.Options()
        options.layout_features = []
        options.hinting = False
        options.notdef_outline = True
        font = subset.load_font(str(fonts / name), options)
        subsetter = subset.Subsetter(options)
        subsetter.populate(unicodes=unicodes)
        subsetter.subset(font)
        subset.save_font(font, str(fonts / "compact" / name), options)
        print(f"compact/{name}: {(fonts / 'compact' / name).stat().st_size} bytes")
    return 0


if __name__ == "__main__":
    sys.exit(main())
