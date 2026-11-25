# Flutter 3.50 widget preview composition

Pinned `lib/src/widget_previews/widget_previews.dart` and the authored
`preview_slice_test.dart` establish default group, newest wrapper outside
earlier wrappers, first theme outermost, immutable builder snapshots,
wrapper clearing, and label/size/text scale/brightness metadata. The
Rust target `flutter350_preview_composition` checks actual retained paint
order and underlying leaf preservation as well as the metadata.
The facade is `incular::widget_previews` and depends on no design system.

This closes the authored wrapper/theme API gap recorded in
[listener-tween-preview-comparisons.md](listener-tween-preview-comparisons.md).
This is only runtime composition. Annotation discovery, generator, scaffold,
host/rendering, file watching/reload, and DTD/IDE integration remain outside
this slice. These authored cases do not count as any of the 113 upstream
tooling registrations.

## Localized preview subtrees

`PreviewLocalizationsData` and `Preview::apply_localizations` install
catalog resources and resolved directionality in the retained preview.
Pinned `preview_localizations_reference_test.dart` exercises the actual
Flutter preview descriptor and the resolver used by its scaffold. Both
sides propagate English, Spanish, and Arabic resources; change inherited
direction without remounting the child; and invoke the list override
before the single-locale override when an explicit locale is supplied.
The Rust environment and preview targets passed 11 tests; the authored
Flutter probe passed both comparisons.

Incular catalogs are not Flutter delegate lists. This probe does not
execute the actual `widget_preview_scaffold/test/localizations_test.dart:50`
registration: the offline SDK cache lacks that tool's design-system and
launcher dependencies. It is not credited as a paired registration.

## Preview scaffold manifest

Three exact hermetic tool registrations passed in the pinned Flutter SDK:
valid manifest fields, initial/stale SDK scaffold regeneration, and root
pubspec changes. Flutter's manifest records manifest and Dart SDK versions
and project pubspec hashes. Incular does not yet generate a preview
scaffold or equivalent cache manifest, so the three results are reference
only and are excluded from the paired count.

To repeat against the SDK's initialized `flutter_tools` package:

```bash
bash audits/flutter-3.50.0-parity/run-reference-linux.sh \
  tooling preview_manifest/preview_manifest_test.dart \
  'PreviewManifest generates a valid manifest' \
  'PreviewManifest identifies widget preview scaffold project needs to be generated' \
  'PreviewManifest identifies root project pubspec has changed'
```

### Default locale selection

Pinned `packages/flutter/test/widgets/app_test.dart:708` passed its six
original preference cases. The paired Rust helper compares the same
inputs and results; `preview_localizations_reference_test.dart` and Rust
also check seven adversarial preferences, including first-supported
ties, promotion of a later same-language exact match, and a region-only
match that actually outranks an unrelated first-supported locale.

Previews and `WidgetsApp` now use the Flutter selection order, rather
than ICU's resource fallback. The app test checks the locale reaches
the generated title. ICU remains the source of text direction,
formatting, and catalog message fallback. BCP-47 extensions remain
available to Rust consumers but do not alter the three-component match.

### Simulated media and actual layout

`Preview::compose` applies the wrapper, theme, optional size, scoped
MediaQuery and localizations in the scaffold's inner-to-outer order.
The scope starts with the nearest MediaQuery, or the retained native
snapshot when the preview is mounted directly. It only replaces the
requested fields. A positive-infinite axis leaves layout sizing to
the child without replacing the simulated MediaQuery extent.

The pinned neutral `preview_media_reference_test.dart` and retained Rust
`flutter350_preview_composition` verify that an unconstrained-width,
60-high preview renders a 40-by-60 leaf, both theme and wrapper observe
the simulated brightness/scale/size, padding changes are inherited
without remounting, and an outer MediaQuery's independent fields are
still visible. The authored probe does not exercise scaffold controls,
including the user-selected brightness override, zoom, or restart.
