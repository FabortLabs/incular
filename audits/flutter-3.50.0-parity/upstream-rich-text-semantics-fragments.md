# Rich text semantic fragments

Pinned Flutter: `3.50.0` (`ab2912035e160d9cad6306df307576da6128ebdb`).

The original `packages/flutter/test/widgets/text_semantics_test.dart:142`
registration mounts a paragraph with a named span among ordinary runs. Its
Incular counterpart checks the same visible string and the same three
semantic children: adjacent ordinary spans merge, the named span retains
its identifier, and trailing ordinary spans merge. The parent is an
unlabeled boundary; it does not announce a duplicate flat label.

The authored pinned `rich_text_fragment_identifier_test.dart` also compares:

- A real inline child interleaved with text fragments; both text fragments
  retain their locale ranges and their IDs survive a rebuild.
- A long spoken alias before the inline child and a named span. Changing
  only the alias length changes pronunciation offsets, not the named
  range's bounds. The native projection uses literal UTF-8 byte indices
  for shaped selection spans and UTF-16 indices for speech attributes.
- An 800-pixel test surface whose paragraph resizes to 120, 94 and 36
  logical pixels. At 94 the placeholder remains but the named range is
  gone. At 36 both are gone, and both return at 120. The synthetic ellipsis
  previously impersonated the first hidden bytes of the named range;
  the renderer's retained source endpoint now excludes that glyph.

Native controls also check that removing the named boundary or turning
semantics off disposes its former fragment IDs. These are mounted
semantics checks, not native assistive-technology or raster tests. The
recognizer-specific originals and the direct renderer assembly fixtures
are tracked separately and not counted here.

```sh
INCULAR_FLUTTER_AHEM_PATH="$PWD/.tmp/flutter/packages/flutter_tools/static/Ahem.ttf" \
  cargo test -p incular-widgets --test flutter350_rich_text_semantics_fragments
```
