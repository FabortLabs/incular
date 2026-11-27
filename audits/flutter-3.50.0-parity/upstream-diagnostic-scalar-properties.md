# Scalar diagnostic descriptions and JSON values

Five pinned Flutter 3.50.0 originals in
`packages/flutter/test/foundation/diagnostics_test.dart` passed: string
and boolean properties, tooltip text, double formatting, and JSON
values for basic types.

The native tests cover the original quoted and unquoted strings,
description overrides, hidden names, missing/empty text, ordinary null
versus a specified null default, tooltip placement, and numeric units.
They JSON-encode and decode each scalar, checking that a present bool,
number, or string retains its JSON type even when the display is
quoted, rounded or decorated. Numeric properties reuse the existing
Flutter-compatible diagnostic precision implementation.

All five property cases and five diagnostic-precision regressions
passed. This adds a typed scalar property API; it does not claim
Flutter's custom/lazy diagnostic objects, flags, or general tree
inspection and formatting.

## Flags, percentages, and special numbers

Five more exact pinned cases passed, including both distinct flag
registrations with the same title. Native tests cover both directions
of label omission, hidden fallback text, typed flag values, and
metadata for an integer with an explicit default, hidden name and
no tree display. Percentages clamp only the readable description,
preserving the original value and appending the requested unit with
one space. Positive and negative infinity are emitted as named JSON
strings rather than silently becoming null.

All ten focused property cases and seven clamp/precision regressions
passed. Lazy computation, exception metadata, and richer diagnostic
tree styles are not part of this slice.
