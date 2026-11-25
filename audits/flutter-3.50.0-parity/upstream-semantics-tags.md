# Semantics: retained node tags

The two exact pinned `semantics_test.dart` originals `:28` and `:43`
compare adding identity-based tags and retaining them in merged
semantics. Incular now exposes explicit framework-only tags without
putting them on the accessibility wire. Cloning preserves identity;
creating a new tag with the same display name does not match the
first, as a non-constant Flutter construction does.

The authored Flutter and retained Rust controls verify that a merge
unions disjoint tags and deduplicates shared ones. Rust also checks the
same retained target loses the superseded tag when its child is
removed. Nine nearby merged/availability tests and both new native
tests passed. This slice does not claim Flutter's `tagForChildren`
propagation or Scrollable's filtering use of those inherited tags.

## Tags for children and merged point dispatch

`Semantics::tag_for_children` crosses layout-only wrappers, combines
tags from multiple annotations, and attaches them to the first semantic
node. They do not leak through its independent boundary or tag the
source annotation itself. Changing or clearing the tag leaves the
child's node ID and independent tags intact. Both the Flutter probe
and retained native test check this sequence.

The exact `semantics_test.dart:946` registration now pairs a
point-addressed tap inside a merged semantic node. The Rust control
also verifies that an outside point is ignored. This retains the
framework's coordinate dispatch, distinct from platform pointer
routing. The runtime controls were run next to strict enabled-state
and action-dispatch frame fixtures.
