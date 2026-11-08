// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::path::Path;

use syn::{Attribute, Item, Visibility};

use super::Violation;

const MESSAGE: &str = "public error types must be `#[non_exhaustive]` (API review)";

pub(crate) fn violations(path: &Path, file: &syn::File) -> Vec<Violation> {
    if !is_library_source(path) {
        return Vec::new();
    }
    let mut found = Vec::new();
    collect(path, &file.items, &mut found);
    found
}

fn is_library_source(path: &Path) -> bool {
    let mut parts = path.components().map(std::path::Component::as_os_str);
    let crate_dir = parts.next().and_then(|part| part.to_str());
    let in_src = parts.next().is_some_and(|part| part == "src");
    in_src && crate_dir.is_some_and(|name| name.starts_with("lunatui") && name != "lunatui-web")
}

fn collect(path: &Path, items: &[Item], found: &mut Vec<Violation>) {
    for item in items {
        match item {
            Item::Enum(item) => check(path, &item.ident, &item.vis, &item.attrs, found),
            Item::Struct(item) => check(path, &item.ident, &item.vis, &item.attrs, found),
            Item::Mod(item) => {
                if let Some((_, nested)) = &item.content {
                    collect(path, nested, found);
                }
            }
            _ => {}
        }
    }
}

fn check(
    path: &Path,
    ident: &syn::Ident,
    vis: &Visibility,
    attrs: &[Attribute],
    found: &mut Vec<Violation>,
) {
    let public = matches!(vis, Visibility::Public(_));
    if public
        && ident.to_string().ends_with("Error")
        && !attrs
            .iter()
            .any(|attr| attr.path().is_ident("non_exhaustive"))
    {
        found.push(Violation::new(path, ident.span().start().line, MESSAGE));
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::violations;

    fn lines(path: &str, source: &str) -> syn::Result<Vec<usize>> {
        let file = syn::parse_file(source)?;
        Ok(violations(Path::new(path), &file)
            .iter()
            .map(|violation| violation.line)
            .collect())
    }

    #[test]
    fn a_public_error_enum_needs_the_attribute() -> syn::Result<()> {
        assert_eq!(lines("lunatui-a/src/lib.rs", "pub enum AError {}\n")?, [1]);
        Ok(())
    }

    #[test]
    fn a_public_error_struct_needs_the_attribute() -> syn::Result<()> {
        assert_eq!(lines("lunatui-a/src/lib.rs", "pub struct AError;\n")?, [1]);
        Ok(())
    }

    #[test]
    fn the_attribute_satisfies_the_check() -> syn::Result<()> {
        let source = "#[derive(Debug)]\n#[non_exhaustive]\npub enum AError {}\n";
        assert_eq!(lines("lunatui-a/src/lib.rs", source)?, []);
        Ok(())
    }

    #[test]
    fn private_and_other_types_are_ignored() -> syn::Result<()> {
        let source = "enum AError {}\npub(crate) struct BError;\npub enum Mode {}\n";
        assert_eq!(lines("lunatui-a/src/lib.rs", source)?, []);
        Ok(())
    }

    #[test]
    fn nested_modules_are_checked() -> syn::Result<()> {
        let source = "mod inner {\n    pub enum AError {}\n}\n";
        assert_eq!(lines("lunatui-a/src/lib.rs", source)?, [2]);
        Ok(())
    }

    #[test]
    fn tests_examples_and_other_directories_are_ignored() -> syn::Result<()> {
        let source = "pub enum AError {}\n";
        for path in [
            "lunatui-a/tests/a.rs",
            "lunatui-a/examples/a.rs",
            "benches/src/a.rs",
            "xtask/src/a.rs",
            "lunatui-web/src/a.rs",
        ] {
            assert_eq!(lines(path, source)?, [], "{path}");
        }
        Ok(())
    }
}
