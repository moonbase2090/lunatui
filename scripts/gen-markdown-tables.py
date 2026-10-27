#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
#
# Regenerates lunatui-text/src/markdown/tables.rs from the Python standard
# library's Unicode database and its HTML5 entity table.
#
#   python3 scripts/gen-markdown-tables.py > lunatui-text/src/markdown/tables.rs

import html.entities
import sys
import unicodedata


def ranges(points):
    out = []
    for p in points:
        if out and out[-1][1] + 1 == p:
            out[-1][1] = p
        else:
            out.append([p, p])
    return out


def main():
    punct = [c for c in range(0x80, 0x110000)
             if unicodedata.category(chr(c))[0] in "PS"]
    space = [c for c in range(0x80, 0x110000)
             if unicodedata.category(chr(c)) == "Zs"]
    print("// Copyright (c) 2026 Moonbase 2090 LLC")
    print("// SPDX-License-Identifier: MIT")
    print()
    print("pub(super) static PUNCTUATION: [(u32, u32); %d] = [" % len(ranges(punct)))
    for lo, hi in ranges(punct):
        print("    (0x%X, 0x%X)," % (lo, hi))
    print("];")
    print()
    print("pub(super) static SPACE: [(u32, u32); %d] = [" % len(ranges(space)))
    for lo, hi in ranges(space):
        print("    (0x%X, 0x%X)," % (lo, hi))
    print("];")
    print()
    pairs = {}
    for name, value in html.entities.html5.items():
        if name.endswith(";"):
            pairs[name[:-1]] = value
    names = sorted(pairs.items(), key=lambda kv: kv[0].encode())
    print("pub(super) static ENTITIES: [(&str, &str); %d] = [" % len(names))
    for name, value in names:
        escaped = "".join("\\u{%X}" % ord(c) for c in value)
        print('    ("%s", "%s"),' % (name, escaped))
    print("];")
    print("unicode %s" % unicodedata.unidata_version, file=sys.stderr)


main()
