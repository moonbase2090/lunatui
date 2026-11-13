# ADR 0008: Named controls in the frame hit map

## Usage

Named control metadata is part of `lunatui-runtime/app`, including the default
runtime configuration. No separate accessibility opt-in is needed. The existing
`accessibility` feature remains an alias that enables `app`.

```rust,ignore
hits.push_control(id, area, z,
    ControlMetadata::new("Save", AccessibilityRole::Button)
        .disabled(false).layer(None));
hits.write_accessibility_json(&mut output, focus.focused())?;
```

`lunatui_field::Field` and `Form` register their controls while rendering through
`RenderCx::hit_map`. `NamedControl` wraps other widgets without adding visual
chrome. A visible control label supplies its name. An unlabeled control requires
`.name()`. Empty and whitespace-only names are hard errors, including in release
builds. Input values cannot supply an implicit name.

## Ownership and flow

`HitMap` owns each frame's IDs, rectangles, exact depth and registration order.
Names, roles, disabled state and optional `LayerId` belong to those same records.
Clearing the map clears the semantics. Scroll views and partially clipped list
items translate and clip their records with the corresponding cell blit.
Completely clipped controls are omitted from the rendered frame registry.

`controls()` returns named records. `find_control(role, name, layer)` reports an
ambiguity error if several records match; it never chooses the first duplicate.
Names can repeat across different roles or layers. Disabled controls remain
queryable, but pointer picking cannot activate them or click through them.

`FocusManager` remains the owner of focus. Callers compare each control ID with
`focused()`, and pass that current ID to the exporter. Only enabled controls
belong in an application's focus ring. Layer reachability remains the owning
layer stack's decision; the registry preserves layer identity and depth without
inferring modality.

Raw `push` remains available for geometric routing that is not a named control.
Legacy `push_with_role` annotations can still be exported, but are excluded from
named lookup because they have no accessible name.

## Format and text boundary

The experimental export is a flat version-2 JSON snapshot, not an accessibility
tree or a platform adapter. Each successful write emits one object and a newline:

```json
{"version":2,"regions":[{"id":7,"role":"button","name":"Save","disabled":false,"layer":null,"area":{"x":1,"y":2,"width":8,"height":1},"z":0,"focused":true}]}
```

Control names are permitted application text. This revises the earlier
version-1 restriction that excluded all application text. Input values, help,
validation messages, surrounding prose and widget contents remain excluded.
`ControlMetadata` has no value member. Applications must use a label or a fixed
accessible name, never copy an editable value into the name. Names are escaped
as JSON strings, including quotes, backslashes, control characters and Unicode.

Only annotated regions appear, in registration order. Coordinates are terminal
cells. Direct registrations retain repeated IDs and empty rectangles; export
does not infer hierarchy or visibility. Legacy role-only annotations have
`name: null`, `disabled: false`, and `layer: null`. Version-1 consumers must adopt
the version-2 schema. Writer errors propagate and may leave a partial snapshot.

## One field model

`lunatui-field` owns both `Field` and `Form`. A field holds the label/name,
required/help/error/disabled state and optional borrowed control widget. A form
chooses the side-by-side layout and constrains each body to `input_rows`, one by
default. The same descriptor can also render as a standalone stacked field.
The old `lunatui-widgets::FormField` descriptor and form implementation are
removed. See [the migration guide](../named-controls.md).

Keeping composition in `lunatui-field` preserves dependency direction:
`lunatui-field` depends on UI tokens and widgets; UI tokens already depend on
widgets. Moving the field into widgets would create a cycle or duplicate token
behavior. All in-tree callers migrate together.

## Alternatives

A separate registry would duplicate geometry and lifecycle. Attaching metadata
to existing hit regions prevents that drift. Keeping two field descriptors would
retain two owners for naming and validation. A platform accessibility tree would
also need actions, hierarchy and lifecycle rules beyond this snapshot format.
JSON uses the standard-library writer without an intermediate document or a
serialization dependency.
