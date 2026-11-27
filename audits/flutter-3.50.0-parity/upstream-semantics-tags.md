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

## Adapter boundary

Flutter's engine semantics update does not carry framework tags.
A retained Rust tag-only update previously yielded a mobile state
event and an unchanged AccessKit node patch, and the mobile snapshot
still carried the tag. Both incremental projections now compare
tag-free node snapshots. The generic bridge snapshot also omits
tags. The retained node still answers tag queries; a subsequent
real enabled-state change still reaches the same native IDs.
Two regression cases failed before the fix and passed afterwards,
alongside 14 focused projection cases. These are adapter-output
checks, not measurements of physical screen-reader timing.

## Immediate list tags stop before a nested scroll boundary

Exact pinned `widgets/semantics_test.dart:54,429` and the two native
`flutter350_semantics_child_tags` ports passed. The ordinary annotation
passes its custom tag to both direct semantic text children but not to
itself. The original list fixture does not mark a node that has implicit
scrolling with the list's two-pane tag, even after a subsequent frame.

The additional pinned and native control verifies the actual shape:
the tagged, indexed list item is a non-scrolling outer node around the
untagged inner scroll node. Before the fix Incular put the index, inherited
tag and scroll semantics on one node. The existing synthetic pane path now
handles nested indexed scroll boundaries and keeps their scroll actions
on the inner node. Point dispatch can also pass through semantics-only
containers to live descendants, as verified in the pinned and native
fixtures. The 27 selected regressions across the two native runs passed.

## Application semantics shutdown and restart

Exact pinned `widgets/semantics_test.dart:21` passed. Incular's focused
application fixture acquires, publishes, releases, and reacquires a
semantics request on the same window, observing the same labeled content
and direction on both starts. The projection removes the retired native
node. Flutter does not schedule a frame merely to release the request;
Incular intentionally wakes its native host to release the held
projection. This remains a partial match for scheduling.
