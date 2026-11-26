# Stylus handwriting commands

Three pinned `services/scribe_test.dart` registrations pair, over
all seven runtime variants. Both capability calls use flutter/scribe
with their exact JSON method and null arguments. The service preserves
false and true and reports the unexpected-null error separately from
either boolean. Starting handwriting sends exactly one command and
accepts a null void result. Native evidence uses the actual serialized
method and reply envelopes.

The fourth registration, unsupported inbound dispatch, passes on
Flutter but remains unpaired because the optional embedder adapter
does not own a global inbound binary-message dispatcher. All 17
selected native checks passed. This describes the channel, not
native stylus recognition on an Android device.
