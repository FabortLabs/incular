# incular-assets

## Architecture and support

| Contract | Status |
| --- | --- |
| Ownership | Font identities, shared font bytes, asset bundles and manifest metadata. |
| API class | application; re-exports and path overrides follow the [architecture contract](https://github.com/FabortLabs/incular/blob/master/system-design/ARCHITECTURE.md). |
| Support | Asset bundles and manifest metadata; raster decoding belongs to incular-image. |

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

## Typed assets and async loading

`AssetHandle<T>` names one typed asset (`AssetId` identity plus a logical
key). `TypedAsset` decodes bundle bytes; `load_typed` reads synchronously,
`load_typed_async` is callback-based and `load_typed_future` returns an
`AssetFuture` (`std::future::Future`, no async runtime required). Loads
report clear `AssetLoadState`s: `Loading`, `Ready`, `Failed`. Built-in
typed assets are `FontAsset` (font source bytes + face/variation metadata,
convertible to `FontHandle` and consumable by `incular-text`'s `FontLoader`)
and `ShaderAsset` (shader source bytes + `ShaderLanguage`); raster images
stay in `incular-image`. Existing byte and string bundle APIs are unchanged.

## Path overrides

`PathOverrides` maps logical keys to physical keys (applied once, no
chains); `PathOverrideBundle` applies a live set to any wrapped bundle
while preserving byte, string, typed, async and manifest APIs, re-keying
errors to the logical key, and evicting both logical and physical keys.

## Hot reload (`hot-reload` feature)

With `hot-reload`, `hot_reload` exposes invalidation hooks:
`AssetBundle::hot_reload_invalidate` / `hot_reload_invalidate_all`, a
process-wide `subscribe`/`invalidate*` broadcast, and `watch` to invalidate
a shared bundle automatically; cached assets reload on the next read.
Builds without `hot-reload` are unaffected.
