# Clipboard presence notifier

Pinned Flutter :1801 and :1827 both pass. The Incular widget service sends
the actual `Clipboard.hasStrings` method on `flutter/platform`, using
`text/plain` and never requesting clipboard contents. The tests decode
the outgoing method and answer with real Flutter JSON method envelopes:

- An error reports a diagnostic and leaves the value Unknown. Refresh
  completes normally; a later successful reply can recover it. A missing
  implementation also reports an error and resets a known value to Unknown.
- The value starts Unknown. An empty clipboard becomes NotPasteable; a
  later nonempty clipboard becomes Pasteable. Equal states do not notify
  listeners again.

A separate authored Flutter probe and native delayed-transport check
verify first-observer refresh, refreshing on Resume but not Inactive,
no further scheduling after the last listener drops, and ignoring a
successful response received after disposal. This extra probe does not
increment the upstream registration count. The comparison covers the
notifier and its host-message contract, not OS clipboard permissions
or a design-system toolbar.
