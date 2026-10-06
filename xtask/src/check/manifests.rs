// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::path::Path;

use super::Violation;

pub(crate) fn violations(_path: &Path, _manifest: &str) -> Vec<Violation> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::violations;

    fn count(manifest: &str) -> usize {
        violations(Path::new("member/Cargo.toml"), manifest).len()
    }

    #[test]
    fn a_member_using_only_workspace_lints_is_clean() {
        assert_eq!(
            count("[package]\nname = \"m\"\n\n[lints]\nworkspace = true\n"),
            0
        );
    }

    #[test]
    fn a_member_without_workspace_lints_is_rejected() {
        assert_eq!(count("[package]\nname = \"m\"\n"), 1);
    }

    #[test]
    fn a_member_overriding_lints_is_rejected() {
        let manifest = "[package]\nname = \"m\"\n\n[lints]\nworkspace = true\n\n[lints.clippy]\npedantic = \"allow\"\n";
        assert_eq!(count(manifest), 1);
    }
}
