# Stylus handwriting commands

Three pinned `services/scribe_test.dart` registrations pair, over
all seven runtime variants. Both capability calls use flutter/scribe
with their exact JSON method and null arguments. The service preserves
false and true and reports the unexpected-null error separately from
either boolean. Starting handwriting sends exactly one command and
accepts a null void result. Native evidence uses the actual serialized
method and reply envelopes.

The fourth pinned registration now pairs through the embedder's direct
inbound messenger. The original raw JSON has a method name and no args;
receiving it without a listener replies null exactly once. Installing a
handler before the deferred reply does not replay it. This is direct host
dispatch, not the engine's buffered channel. Nineteen selected native
channel tests passed. The comparison does not claim real handwriting
recognition on an Android device.
