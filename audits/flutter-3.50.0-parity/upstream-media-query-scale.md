# Flutter 3.50 MediaQuery scaling and scope

Pinned `widgets/media_query_test.dart:1295,1320,1700` passed. The
Rust environment target checks fallback scale 1 and light brightness,
nested scale 4 and dark brightness, and retained dependency counters:
a size, scale, or DPR subscriber rebuilds only when that specific
aspect changes; a whole-query subscriber sees all changes.

Previously, the aspect lookup reported the nested text scale correctly,
but paragraph layout ignored it and read the native runtime value.
The retained text pipeline now takes the nearest MediaQuery scale,
then falls back to the window scale without a scope. An explicit
scaler is still authoritative. Both the authored pinned
`scoped_text_scale_reference_test.dart` and the Rust environment target
check measured text width/height at factors 1 and 2, zero width at
factor 0, and unchanged explicit text size without remounting. Flutter's
nonlinear system scaler and the full aspect matrix are not covered here.

## Scoped accessibility and text overrides

Pinned :1345, :1370, :1395, and :1421 establish false defaults
outside the scope and true values inside for high contrast, switch
labels, announcement support, and bold text. Rust checks all four
through retained builders, then changes one and verifies only that
reader is called again. Related supported fields now use the same
nearest-scope aspect mechanism; the full Flutter aspect variant is
still open.

Pinned :1233 and Rust verify that applying text-style overrides replaces
four inherited values of 9 with 2, then clears all four when passed
null/None. Rust also verifies the retained paragraph really receives
bold, line-height, letter spacing and word spacing, and returns to its
original style when the overrides clear. Paragraph spacing remains an
inherited value for clients to apply, not an implicit paragraph layout
rule for the neutral Text widget.

## Ancestor lookup and individual dimensions

Pinned :48, :89, :111, and :129 passed. Rust checks that both lookups
read the closest real scope, that the optional form returns no value
when unscoped, and that the strict form fails with the intended
diagnostic instead of inventing media data. :48 is partial because
Rust has a panic with the diagnostic rather than FlutterError's
full ownership chain and View hint.

The authored pinned `media_query_dimensions_reference_test.dart`
and paired Rust target verify eight strict/optional subscribers.
Width-only and height-only changes do not invalidate the other axis.
Square is portrait, orientation only invalidates on that transition,
and brightness alone does not invalidate any dimension reader.
The original full aspect variant remains separate.
