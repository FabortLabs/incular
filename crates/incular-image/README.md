# incular-image

## Architecture and support

| Contract | Status |
| --- | --- |
| Ownership | Raster image identities, decoding and CPU image caches. |
| API class | application; re-exports and path overrides follow the [architecture contract](https://github.com/FabortLabs/incular/blob/master/system-design/ARCHITECTURE.md). |
| Support | Available; CPU cache budgets, eviction, and release are implemented. Shared GPU eviction stays in W2. |

Renderer-neutral raster image resources for Incular. This crate owns immutable
decoded RGBA8 images, source metadata, byte/file decoding, and the
application-owned decode cache. It intentionally owns no GPU texture, painter,
or widget state.

`ImageHandle` is cheap to clone and identified by a stable `ImageId`; renderers
use that identity to retain backend resources while widgets and display lists
remain renderer independent. PNG, JPEG, and WebP decoding happens at load
time, and generated RGBA8 pixels are validated before a handle is created.

`AssetImage` resolves density from the packaged asset manifest, using the
explicit bundle, image configuration, or root bundle in that order.
`ExactAssetImage` reads its specified key and scale without a manifest.
The resolved bundle key names the physical file and carries the density
used to map source pixels into logical image size. Unbundled native file
paths also support sibling density directories.

A mounted native `FileImage` resolves asynchronously using the host's
existing blocking executor. The path and scale identify its shared cache
entry. Empty files can be retried immediately after the writer fills them;
a nonempty decode failure is replayed until the key is evicted. Explicit
`FileImage::load` and headless resolution remain synchronous.

The UI's `BundleImageCache` deduplicates pending loads and keeps recently
decoded bundle and hosted file images within independent count and byte limits.
A runtime memory-pressure message drops those retained entries. It does not
invalidate image handles already in use.
