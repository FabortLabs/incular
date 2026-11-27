# Raw keyboard listener lifetime and event source

The pinned raw listener cases :10 and :73 and typed listener :66
passed. The exact no-input rebuild/unmount case pairs. Native
mounted tests also send left Meta down/up as raw-only, typed-only,
combined and portable messages. A raw observer receives actual or
mirrored raw events, not a typed-only key. Removed listeners receive
no more events and their old focus node is no longer primary.

The two defunct-listener registrations remain partial here because
the native lifetime test uses the portable raw-message boundary
rather than decoding Fuchsia's originating payload. Focused and
adjacent runs passed: nine listener, eight focus-routing, and five
focus-key-stage tests.

## Raw observers run before a child consumes focus keys

A paired authored reference from pinned Flutter and a native mounted
test expose a distinct bug: an ancestor `RawKeyboardListener` with
descendant focus must log raw down/up before the child's typed
handler consumes each event. The initial native trace contained
only the typed callbacks. Raw listeners now observe the raw message
before consuming hardware/focus routes. A typed-only message and
an unmounted listener remain silent. Normal deprecated
`Focus.onKey` still bubbles with its node's typed callback.
The authored regression does not add an original registration.
One focused and 29 adjacent runtime tests passed.

## Deprecated and typed Focus handlers cooperate

The pinned `focus_manager_test.dart`:264 registration passed in both
exact transit variants, RawKeyEvent and ui.KeyData then RawKeyEvent.
Mounted native tests use three focus nodes and a decoded Digit1
record. Both handlers on a node run in typed-then-raw order. Ignored
results bubble to the outer ancestor; either handled result stops
after that node. SkipRemainingHandlers also stops there but returns
the key to the host. Both transit modes exercise all four traces.
Nine focused and 17 adjacent native tests passed. The two variants
are one upstream registration.
