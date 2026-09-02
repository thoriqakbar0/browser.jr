# Screenshot pipeline

Screenshot work has two internal boundaries. The session prepares browser-owned paint evidence, and a raster process converts that evidence into image bytes.

The supported capture targets, paint coverage, file behavior, and visible failures belong to [capture screenshot](../inspection/capture-screenshot.md).

## Prepare capture evidence

[[src/session.rs#PrepareScreenshot]] resolves the request against the current page and returns [[src/screenshot.rs#PreparedScreenshot]]. [[src/screenshot.rs#CaptureTarget]] and [[src/screenshot.rs#CaptureRect]] carry the requested and resolved bounds.

Preparation collects the geometry and paint evidence needed by the raster boundary. The result is either one complete supported scene or an error owned by the session request.

## Build the paint scene

[[src/screenshot.rs#PaintScene]] contains capture bounds and ordered [[src/screenshot.rs#PaintCommand]] values. The scene is an engine result. It does not contain PNG encoding or CLI file behavior.

This boundary lets the session decide whether its evidence is complete before any image allocation starts.

## Rasterize on demand

[[src/screenshot.rs#OnDemandRasterProcess]] creates a [[src/screenshot.rs#RasterProcess]] only for the first render. [[src/screenshot.rs#SoftwareRasterProcess]] implements the current raster path.

The raster module checks allocation and paint budgets before it constructs [[src/screenshot.rs#RasterImage]]. [[src/screenshot.rs#encode_png]] converts the validated image into PNG bytes.

## Ownership

The session owns preparation because the scene depends on the current page. The CLI session owns raster process lifetime because normal inspection and actions do not need image work.

[[decisions#Rasterization is lazy and bounded]] records the reason for this split. [Capture screenshot](../inspection/capture-screenshot.md) remains the behavior owner.
