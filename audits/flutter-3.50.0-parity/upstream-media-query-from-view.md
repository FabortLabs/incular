# MediaQuery from a live view

Pinned Flutter 3.50.0 `packages/flutter/test/widgets/media_query_test.dart`
registrations 428, 490, and 551 passed. The native tests mount a
MediaQuery over a real retained ViewController and record builds from
MediaQuery.of:

- Without an inherited platform override, changes to text scale,
  brightness, and accessibility each reach the reader exactly once.
  A device-pixel-ratio change also updates logical size.
- With an override, those same three platform notifications do not
  rebuild the reader. A metrics change still does.
- Replacing the parent scope updates the effective platform data
  without changing which view supplies metrics.

The view supplies its own padding, keyboard insets, gesture insets,
and other view geometry. Reading the same effective environment
does not invalidate its descendants. An authored native case also
checks logical inset provenance, explicit zero scale, and optional
typography defaults. It adds no upstream credit. The isolated native
run passed 22 cases, including positioning and window environment
regressions. This does not claim complete platform notification or
display-feature parity for every host.

## Host-provided keyboard insets in WidgetsApp

The authored pinned Flutter control changes the actual test view's
physical insets, scaled by device pixel ratio, while one tooltip stays
open. The inherited logical inset changes from 0 to 200 and back, and
the measured popup moves from y=380 to y=340 and back. No synthetic
MediaQuery wrapper is inserted into the Flutter fixture.

Incular reproduced the stale position with a default WidgetsApp:
its root installed an immutable MediaQuery that masked new host
metrics. The default now follows the retained view and the same
mounted popup moves when the host updates its environment. An
explicitly supplied app environment remains a fixed override. The
native comparison verifies the element identity does not change.
All 75 targeted native checks passed across app shell, environment,
and popup behavior. The authored controls add no upstream credit.
