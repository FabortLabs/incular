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
This is only runtime composition. Locale/delegate integration, annotation
discovery, generator, scaffold, host/rendering, file watching/reload,
and DTD/IDE integration remain outside this slice. These authored cases
do not count as any of the 163 upstream tooling registrations.
