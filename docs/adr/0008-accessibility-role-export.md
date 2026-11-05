# ADR 0008: Export optional roles from the frame hit map

## Usage

Enable `lunatui-runtime/accessibility`, then register semantic regions in the
same map used for pointer routing:

```rust,ignore
hits.push_with_role(id, area, z, AccessibilityRole::Button);
hits.write_accessibility_json(&mut output, focus.focused())?;
```

The experimental export is a flat JSON snapshot, not an accessibility tree or
a platform accessibility adapter. It fulfills the role-export step in the PRD.

## Ownership and flow

HitMap already owns each frame's IDs, rectangles, depth and registration order.
An optional role belongs to that same region, so clearing the map clears its
semantics too. FocusManager continues to own focus; the exporter takes its
current ID without changing it. Existing `push` calls remain unannotated.

The off-by-default `accessibility` feature implies `app`. Without the feature,
HitRegion has no role field and the export API is absent. The implementation
lives beside HitMap in `hit/accessibility.rs`.

## Format

Each successful write emits one JSON object and a newline:

```json
{"version":1,"regions":[{"id":7,"role":"button","area":{"x":1,"y":2,"width":8,"height":1},"z":0,"focused":true}]}
```

Only explicitly annotated regions appear, in registration order. Coordinates
are terminal cells. Repeated IDs, overlapping regions and empty rectangles
retain the meaning they have in HitMap; export does not infer visibility or
hierarchy. Every region matching the supplied focus ID is marked focused.
Role strings come from an enum; the format contains no application text or
input values. Writer errors propagate, and a failed write may leave a partial
snapshot in the destination.

## Alternatives

A separate semantic registry would duplicate geometry and require callers to
synchronize insertion and clearing across two collections. Attaching roles to
the existing records prevents that drift. A platform adapter or accessibility
tree would require labels, actions, hierarchy and lifecycle rules outside this
experimental export's scope. JSON uses a standard-library writer, without
adding a serialization dependency or allocating an intermediate document.
