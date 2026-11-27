# Package-qualified font styles and actual shaping

The three exact pinned originals at `painting/text_style_test.dart:340,
375,400` pass. Rust keeps the `packages/p/` family identity through
cloning, apply, merge, and both animation endpoints. Absent, ordered
and explicitly empty fallback lists remain distinguishable. The package
merge registration pairs. The other two are partial because Flutter
also checks its engine TextStyle debug strings and nullable family
descriptors, rather than Rust's typed shaping inputs.

The authored Dart probe registers a real Inter fixture under a named
package alias. After loading, the primary family changes measured
width and the fallback selects the same face for Ψ. The synthetic
Flutter test font manager substitutes Ahem for unknown names; because
Ahem already has the Latin control glyph, it wins that control. The
fallback assertion therefore uses Ψ, which Ahem lacks and Inter has.

Rust checks selected glyph-run face bytes, not just width or family
names. One live engine drops the cached fallback after late
registration, resolves package-qualified primary and fallback runs,
and a second engine distinguishes the same short name in another
package. A mounted Text also picks up the new face without replacing
its render object. Forty-five distinct focused Rust checks and the
authored Flutter probe passed. This does not compare GPU raster pixels
or provide a platform asset-manifest loader.

## Replacing a package qualifier

The pinned `painting/text_style_test.dart:669` passed. The native
tests distinguish otherwise identical styles from different packages
in equality and hashing. Copying or applying a replacement package
adds the qualifier to an unqualified name and replaces an existing
qualifier without nesting it. A control preserves the fallback order
and the distinction between omitted and explicitly empty lists.
