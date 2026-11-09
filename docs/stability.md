# Stability policy for 1.0

This is what counts as a breaking change after 1.0, and how the API is shaped so that most additions are not. It comes out of the [API review](api-review.md). The conventions in [`conventions.md`](conventions.md) say how code is written; this page says what callers can rely on.

## What is breaking

After 1.0 a change to a public item in a library crate needs a major version if it can stop a correct downstream program from compiling or change what it does. That means:

- Removing, renaming or moving a public item, or making one private.
- Changing a signature: parameter or return types, generics, `const` or `unsafe`, or a trait bound a caller must meet.
- Adding a required method, associated type or constant to a trait (see below).
- Removing a trait implementation a caller may use.
- Raising the minimum supported Rust version.
- Changing documented behavior: output bytes of an encoder, a default value, a layout result for the same input.

Not breaking, and allowed in a minor version:

- Adding a public item, a method with a default body on a trait, a variant to a `#[non_exhaustive]` enum, or a field to a `#[non_exhaustive]` struct.
- Adding a feature, or an item behind a feature, as long as enabling it does not change an existing item (features are additive).
- Adding a trait implementation for a type, unless it makes an existing call ambiguous.
- Fixing behavior that contradicts the documentation, with a changelog entry.

## How the API is shaped

- **Open enums are `#[non_exhaustive]`.** An enum that names a set the outside world defines or extends (protocol and capability values, input kinds, styles, tree nodes, fonts and patterns) is open. Downstream code matches it with a wildcard arm. An enum that is a fixed choice (an axis, a direction, a two-way mode, a value the packed cell format fixes) stays exhaustive so callers can match it without a wildcard; adding a value to one is a breaking change.
- **Option and result structs are `#[non_exhaustive]`.** A struct that holds options, or that the library produces and callers read, cannot be built with a struct literal outside its crate and cannot be matched without `..`. Options are built from `Default` or a constructor plus setters; results are read by field. Complete value types (`Size`, `Rect`, `Style`, `Padding` and the like) stay open to construction.
- **Error types are `#[non_exhaustive]`.** `cargo xtask check` fails on any public type named `*Error` without the attribute.
- **Traits are not sealed.** The traits in this workspace are meant to be implemented by downstream code, so a new required method is breaking. A method added after 1.0 has a default body.
- **Features are additive.** Enabling a feature never removes or changes an item. A public item behind a feature is public API when that feature is on.
- **Each `#[non_exhaustive]` addition has a compile-fail test.** A `compile_fail` doctest in the type's documentation matches every value, or builds or lists every field, the way downstream code would. Removing the attribute fails the test.

## Outside the promise

- The accessibility export in `lunatui-runtime` (feature `accessibility`) is documented as experimental and is not covered until that note is removed.
- The bench, test-helper and web-sample crates are not published API.
- Items documented as internal, and anything not reachable from a crate root or a public module.
- The exact bytes of recorded captures and the `site/` sandbox build.

## When a breaking change is needed

A breaking change before 1.0 lands with a changelog line that starts with "Breaking:", names the types, and shows the one-line fix for downstream code (a wildcard arm, a setter call, a trailing `..`). After 1.0 the same line goes under the next major version, and the old item is first marked `#[deprecated]` for at least one minor release when that is possible.
