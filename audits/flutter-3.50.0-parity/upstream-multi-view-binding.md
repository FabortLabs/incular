# Multi-view ownership, metrics, painting and input

All six pinned `rendering/multi_view_binding_test.dart` cases pass.
Three pair: live-view enumeration follows insertion and removal;
the 400×600 / 2.5 view changes to 100×100 logical pixels when it
receives 300×300 / 3 metrics, without touching its neighbor; and
semantic Copy and Tap go to the addressed live view only. The
native semantic test uses real mounted callbacks and a platform
semantic projection, and rejects an unknown view.

The remaining three are partial: duplicate native view IDs are
unrepresentable and stale closes return false, rather than Flutter's
manual registration assertions. The host test independently draws
both live views, then one and then none as they close; this is not
Flutter's single global compositor callback. The native pointer
test routes two identical coordinates to their respective mounted
targets and ignores an unknown view, but does not expose Flutter's
synthetic binding entry on the hit-test path.

All eighteen focused native window, semantics and input cases
passed. These tests dispatch simulated native events to application
windows and do not open actual desktop windows.
