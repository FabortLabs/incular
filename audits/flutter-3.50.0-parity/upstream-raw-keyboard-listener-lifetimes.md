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
