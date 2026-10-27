# lunatui-image

Shared validated image data and pixel-space fitting geometry for Lunatui.
Terminal transports and widget rendering are provided by other crates.

`ImageSource::rgba8` accepts immutable RGBA8 pixels with rows from top to
bottom. It checks dimensions and byte length before retaining the source.
`ImageGeometry::fit` maps that source into a pixel viewport with `Contain`,
`Cover`, `Stretch`, or `Native` behavior. A geometry contains both the source
crop and the destination rectangle so every output protocol can use the same
fit result.
