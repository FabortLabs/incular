# Display P3 and extended sRGB conversions

Exact pinned `painting/colors_test.dart:630,651,668` passed. The
`flutter350_wide_color` native tests repeat the P3 teal, pure P3 green,
and the sRGB → P3 → extended-sRGB round trip with the original
per-channel tolerance. The conversion decodes transfer functions,
transforms D65 primaries in linear light, and encodes with a signed
transfer function. Extended sRGB retains negative and superwhite
channels until the caller chooses a bounded result.

A separate authored pinned probe verifies packing the converted
extended-sRGB teal as `0x8000CED5`, and the native conversion packs
the exact same bytes with unchanged straight alpha. The direct
P3 → bounded-sRGB route in this Flutter pin instead clips its input
without applying the wrapped matrix; that branch produced
`0x801ECAD3`. Incular deliberately converts before clipping there;
the discrepancy is outside the three credited originals.

This is CPU-side value conversion. It does not advertise a P3
swapchain or change the wgpu renderer's sRGBA8 color type. The
focused native tests and the façade check without default features
passed.
