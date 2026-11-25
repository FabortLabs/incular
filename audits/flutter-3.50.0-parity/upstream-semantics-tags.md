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
