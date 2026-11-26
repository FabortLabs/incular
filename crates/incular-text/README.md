# incular-text

## Architecture and support

| Contract | Status |
| --- | --- |
| Ownership | Font selection, shaping, metrics, text layout and editing values. |
| API class | application; re-exports and path overrides follow the [architecture contract](https://github.com/FabortLabs/incular/blob/master/system-design/ARCHITECTURE.md). |
| Support | Available; controller observation consolidates in C/G. |

Renderer-independent font selection, OpenType shaping, Unicode line breaking,
metrics, baselines, and a bounded logical text-layout cache. Parley 0.7,
Fontique, and HarfRust are the single authority for discovery, fallback, bidi,
and shaping. This crate emits font handles plus positioned glyph runs; it does
not own GPU state or rasterize glyphs. Layout cache keys use metric inputs, not
foreground color or display DPI: a 16 logical-pixel paragraph remains a 16
logical-pixel paragraph at all window scales. `incular-wgpu` independently
rasterizes the resulting glyph IDs at physical DPI and caches those physical
mask variants, preserving logical layout across DPI changes.

The renderer uses each logical glyph run to request a DPI-aware physical
grayscale mask from its cached `fontdue` font. Raster cache identity is owned
by the renderer; shaping, advances, clusters, logical baselines, wrapping, and
caret geometry remain here. Scroll and compositor transforms are renderer
placement only and never alter the shaped text.

## Word selection and line breaking

On Windows 10 version 1903 and later, Incular uses Windows ICU for Unicode
word boundaries and dictionary line breaks in Thai, Lao, Khmer and Burmese.
The `bundled-dictionaries` feature selects ICU4X data distributed with the
application, including on systems without `icu.dll`. Other native targets
use the bundled dictionaries. The web host uses the browser’s word segmenter.

## Font resolution and fallback

`TextEngine` owns one Parley `FontContext` and reusable `LayoutContext`.
Fontique discovers system and registered application fonts; Parley resolves
generic, named, and supplied fallback families, applies coverage-aware fallback,
and shapes/bidi-reorders/runs line breaking in one layout pass. Each physical
face produces a renderer-neutral glyph run with a stable byte-plus-collection
face `FontId`. Registration advances the collection generation and invalidates
the bounded layout cache. Fontdue remains only the WGPU R8 alpha-mask
rasterizer; color-glyph formats are outside this path.

## Paragraph bounds and overflow

`Text` and `RichText` expose `soft_wrap`, `max_lines`, and `TextOverflow`
without leaking Parley layout types. The same `TextLayoutOptions` reaches the
retained widget tree. `Clip` restricts retained paint to the laid-out bounds,
`Visible` preserves the full logical layout, and `Ellipsis` shapes U+2026 with
Parley after grapheme-safe truncation, including mixed-direction text.

Use `TextEditingDelta::try_apply` or `TextEditingController::try_apply_delta`
when malformed offsets need to be reported. A reversed, out-of-range, or
misaligned UTF-8 replacement range does not edit the value or run formatters.

Use `TextScaler::try_clamp(min, max)` to cap or floor a linear scale before
laying out text. Invalid bounds return `TextScaleBoundsError`. No scaling
and a linear factor of one are equal as scaling policies.
