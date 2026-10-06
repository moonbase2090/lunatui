// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::path::Path;

use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{AttrStyle, Attribute, Expr, ImplItem, Item, Meta, TraitItem, Visibility};

use super::Violation;

const PRIVATE_DOC_MESSAGE: &str = "`///` docs are only allowed on public API items (M14)";

const CRATE_DOC_MESSAGE: &str =
    "crate docs must be `#![doc = include_str!(\"README.md\")]` with a README path (M14)";

pub(crate) fn violations(path: &Path, file: &syn::File) -> Vec<Violation> {
    let mut placement = DocPlacement {
        path,
        function_depth: 0,
        found: Vec::new(),
    };
    placement.check_inner(&file.attrs);
    placement.visit_file(file);
    placement.found
}

struct DocPlacement<'p> {
    path: &'p Path,
    function_depth: usize,
    found: Vec<Violation>,
}

impl DocPlacement<'_> {
    fn check_outer(&mut self, attrs: &[Attribute], public: bool) {
        if public && self.function_depth == 0 {
            return;
        }
        for attr in attrs
            .iter()
            .filter(|attr| is_doc(attr) && matches!(attr.style, AttrStyle::Outer))
        {
            self.found.push(Violation::new(
                self.path,
                attr.span().start().line,
                PRIVATE_DOC_MESSAGE,
            ));
        }
    }

    fn check_inner(&mut self, attrs: &[Attribute]) {
        for attr in attrs.iter().filter(|attr| {
            is_doc(attr) && matches!(attr.style, AttrStyle::Inner(_)) && !includes_readme(attr)
        }) {
            self.found.push(Violation::new(
                self.path,
                attr.span().start().line,
                CRATE_DOC_MESSAGE,
            ));
        }
    }

    fn check_item(&mut self, item: &Item) {
        match item {
            Item::Const(item) => self.check_outer(&item.attrs, is_public(&item.vis)),
            Item::Enum(item) => {
                let public = is_public(&item.vis);
                self.check_outer(&item.attrs, public);
                for variant in &item.variants {
                    self.check_outer(&variant.attrs, public);
                    for field in &variant.fields {
                        self.check_outer(&field.attrs, public);
                    }
                }
            }
            Item::ExternCrate(item) => self.check_outer(&item.attrs, is_public(&item.vis)),
            Item::Fn(item) => self.check_outer(&item.attrs, is_public(&item.vis)),
            Item::Impl(item) => {
                self.check_outer(&item.attrs, false);
                let inherent = item.trait_.is_none();
                for impl_item in &item.items {
                    let (attrs, public) = impl_item_docs(impl_item);
                    self.check_outer(attrs, inherent && public);
                }
            }
            Item::Macro(item) => {
                let exported = item.attrs.iter().any(|a| a.path().is_ident("macro_export"));
                self.check_outer(&item.attrs, exported);
            }
            Item::Mod(item) => {
                self.check_outer(&item.attrs, is_public(&item.vis));
                self.check_inner(&item.attrs);
            }
            Item::Static(item) => self.check_outer(&item.attrs, is_public(&item.vis)),
            Item::Struct(item) => {
                self.check_outer(&item.attrs, is_public(&item.vis));
                for field in &item.fields {
                    self.check_outer(&field.attrs, is_public(&field.vis));
                }
            }
            Item::Trait(item) => {
                let public = is_public(&item.vis);
                self.check_outer(&item.attrs, public);
                for trait_item in &item.items {
                    self.check_outer(trait_item_attrs(trait_item), public);
                }
            }
            Item::TraitAlias(item) => self.check_outer(&item.attrs, is_public(&item.vis)),
            Item::Type(item) => self.check_outer(&item.attrs, is_public(&item.vis)),
            Item::Union(item) => {
                self.check_outer(&item.attrs, is_public(&item.vis));
                for field in &item.fields.named {
                    self.check_outer(&field.attrs, is_public(&field.vis));
                }
            }
            Item::Use(item) => self.check_outer(&item.attrs, is_public(&item.vis)),
            Item::ForeignMod(item) => self.check_outer(&item.attrs, false),
            _ => {}
        }
    }
}

impl<'ast> Visit<'ast> for DocPlacement<'_> {
    fn visit_item(&mut self, item: &'ast Item) {
        self.check_item(item);
        visit::visit_item(self, item);
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.function_depth += 1;
        visit::visit_block(self, block);
        self.function_depth -= 1;
    }
}

fn is_doc(attr: &Attribute) -> bool {
    attr.path().is_ident("doc") && matches!(attr.meta, Meta::NameValue(_))
}

fn includes_readme(attr: &Attribute) -> bool {
    let Meta::NameValue(name_value) = &attr.meta else {
        return false;
    };
    let Expr::Macro(include) = &name_value.value else {
        return false;
    };
    include.mac.path.is_ident("include_str")
        && include.mac.tokens.to_string().ends_with("README.md\"")
}

fn is_public(visibility: &Visibility) -> bool {
    matches!(visibility, Visibility::Public(_))
}

fn impl_item_docs(item: &ImplItem) -> (&[Attribute], bool) {
    match item {
        ImplItem::Const(item) => (&item.attrs, is_public(&item.vis)),
        ImplItem::Fn(item) => (&item.attrs, is_public(&item.vis)),
        ImplItem::Type(item) => (&item.attrs, is_public(&item.vis)),
        ImplItem::Macro(item) => (&item.attrs, false),
        _ => (&[], false),
    }
}

fn trait_item_attrs(item: &TraitItem) -> &[Attribute] {
    match item {
        TraitItem::Const(item) => &item.attrs,
        TraitItem::Fn(item) => &item.attrs,
        TraitItem::Type(item) => &item.attrs,
        TraitItem::Macro(item) => &item.attrs,
        _ => &[],
    }
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
