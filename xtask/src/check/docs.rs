// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::path::Path;

use super::Violation;

pub(crate) fn violations(_path: &Path, _file: &syn::File) -> Vec<Violation> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::violations;

    fn violation_lines(source: &str) -> syn::Result<Vec<usize>> {
        let file = syn::parse_file(source)?;
        Ok(violations(Path::new("a.rs"), &file)
            .iter()
            .map(|violation| violation.line)
            .collect())
    }

    #[test]
    fn public_items_and_fields_may_have_docs() -> syn::Result<()> {
        let source =
            "/// A.\npub fn a() {}\n/// B.\npub struct B {\n    /// F.\n    pub f: u8,\n}\n";
        assert_eq!(violation_lines(source)?, []);
        Ok(())
    }

    #[test]
    fn private_items_may_not_have_docs() -> syn::Result<()> {
        assert_eq!(violation_lines("/// A.\nfn a() {}\n")?, [1]);
        Ok(())
    }

    #[test]
    fn restricted_visibility_counts_as_private() -> syn::Result<()> {
        assert_eq!(violation_lines("/// A.\npub(crate) fn a() {}\n")?, [1]);
        Ok(())
    }

    #[test]
    fn private_fields_may_not_have_docs() -> syn::Result<()> {
        assert_eq!(
            violation_lines("/// B.\npub struct B {\n    /// F.\n    f: u8,\n}\n")?,
            [3]
        );
        Ok(())
    }

    #[test]
    fn variants_take_the_visibility_of_their_enum() -> syn::Result<()> {
        let source =
            "/// E.\npub enum E {\n    /// V.\n    V,\n}\nenum P {\n    /// W.\n    W,\n}\n";
        assert_eq!(violation_lines(source)?, [7]);
        Ok(())
    }

    #[test]
    fn items_of_a_public_trait_may_have_docs() -> syn::Result<()> {
        assert_eq!(
            violation_lines("/// T.\npub trait T {\n    /// M.\n    fn m(&self);\n}\n")?,
            []
        );
        Ok(())
    }

    #[test]
    fn inherent_methods_follow_their_own_visibility() -> syn::Result<()> {
        let source =
            "impl S {\n    /// New.\n    pub fn new() {}\n    /// Helper.\n    fn helper() {}\n}\n";
        assert_eq!(violation_lines(source)?, [4]);
        Ok(())
    }

    #[test]
    fn impl_blocks_and_trait_impl_items_may_not_have_docs() -> syn::Result<()> {
        let source =
            "/// Impl.\nimpl Clone for S {\n    /// Clone.\n    fn clone(&self) -> Self { S }\n}\n";
        assert_eq!(violation_lines(source)?, [1, 3]);
        Ok(())
    }

    #[test]
    fn items_inside_function_bodies_may_not_have_docs() -> syn::Result<()> {
        assert_eq!(
            violation_lines("pub fn f() {\n    /// Inner.\n    pub fn g() {}\n}\n")?,
            [2]
        );
        Ok(())
    }

    #[test]
    fn crate_docs_must_include_a_readme() -> syn::Result<()> {
        assert_eq!(
            violation_lines("#![doc = include_str!(\"../README.md\")]\n#![doc = \"Text.\"]\n")?,
            [2]
        );
        Ok(())
    }
}
