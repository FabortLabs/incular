# Services standard binary codecs

## Standard message values

The four exact `Standard message codec` registrations in
`services/message_codecs_test.dart` passed on pinned Flutter 3.50.0 and
the native `flutter350_standard_message_codec` target passes all six tests.

The tests compare the 253/254 and 65535/65536 size-prefix boundaries,
actual 8-byte double alignment, scalar value tags, nested lists, and
maps with mixed key types. A separate pinned reference verifies exact
UTF-8 and nested-null bytes and decoding a slice of a larger buffer.
The native host byte order, the codec's null-message distinction, and
IEEE infinity, NaN and signed-zero handling are retained.

This is the standard data encoding, not a platform channel transport.
Rust keeps ordered map entries instead of normalizing arbitrary Dart
key equality, bounds nesting, and checks requested lengths before
allocating. Dart subclass extension tags and web number coercions are
not claimed.

## Standard method calls and envelopes

The four exact `Standard method codec` registrations passed on pinned
Flutter, and both focused Rust codec targets passed (12 tests total).
The tests preserve mixed string/integer map keys, error code/message/details,
no message, and an optional native stack trace read directly from bytes.
The native target separately tests aligned arguments, a null success reply,
and malformed call/envelope fields. This is a codec API only; it does not
add a service transport or Dart's codec subclassing mechanism.

## Binary, UTF-8 and JSON messages

All six exact originals in the Binary codec, String codec and JSON
message codec groups passed on pinned Flutter; all six native
counterparts passed. Raw bytes use the exact borrowed view, and the
string codec neither reads beyond a slice nor conflates empty with
absent. JSON tests cover scalar and nested data and the exact UTF-8
bytes when a four-byte emoji crosses the reference encoder's 256-byte
chunk boundary. Native JSON uses serde_json; its internal chunking
is not presented as a match.

## JSON method envelopes and diagnostics

Both pinned JSON method envelope registrations pass, preserving an
optional native stack trace and an absent trace. Native controls also
verify missing arguments, ordinary success and malformed error fields.
The pinned standalone diagnostic registration matches display output for
a null-argument MethodCall, a PlatformException with no optional fields,
and a MissingPluginException with no message. General debug formatting
of compound arguments is not claimed. All 16 tests in the three
focused JSON method, standard method, and simple-message targets passed.
