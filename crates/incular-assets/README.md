# incular-assets

## Architecture and support

| Contract | Status |
| --- | --- |
| Ownership | Font identities, shared font bytes and asset bundles. |
| API class | application; re-exports and path overrides follow the [architecture contract](https://github.com/FabortLabs/incular/blob/master/system-design/ARCHITECTURE.md). |
| Support | Asset bundles and manifest metadata; raster decoding belongs to `incular-image`. |

Font handles, asset bundles (Flutter's `AssetBundle`: in-memory,
directory-backed, caching, and network bundles over an application-supplied
fetcher, since this crate takes no HTTP client) and future non-raster resource foundations for
Incular. Raster
image identities, decoding, and caching live in the dedicated `incular-image`
crate, so renderers and widgets can depend on images without pulling in future
asset categories.

`AssetManifest` reads the native binary or web JSON/base64 manifest through
the bundle, lists logical keys, and materializes resolution variants on demand.
`CachingAssetBundle` shares the decoded manifest among simultaneous callers
and evicts it with the corresponding structured asset.
