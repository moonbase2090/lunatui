#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Create the small standalone Rust workspace used by the live PTY recording."""
from pathlib import Path
import sys

root = Path(sys.argv[1])
marker = root / ".lunatui-terminal-fixture"
if root.exists() and any(root.iterdir()) and not marker.is_file():
    raise SystemExit("Refusing to overwrite a directory that is not a generated terminal fixture")
root.mkdir(parents=True, exist_ok=True)
marker.write_text("terminal-demo-v1\n")
(root / "src").mkdir(exist_ok=True)
(root / "Cargo.toml").write_text(
    '[package]\nname = "orbit-demo"\nversion = "0.1.0"\nedition = "2024"\n\n[workspace]\n'
)
(root / "src/lib.rs").write_text("""pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::add;

    #[test]
    fn adds_positive() {
        assert_eq!(add(2, 3), 5);
    }

    #[test]
    fn adds_negative() {
        assert_eq!(add(-4, 1), -3);
    }
}
""")
