# JSON method channel

Pinned Flutter 3.50.0 `services/platform_channel_test.dart`.

## Outbound method calls

The exact registrations :49, :86, :124, :139, :165, and :182 passed.
Native tests compare the sent JSON method name and arguments on the
named channel. Success keeps the returned string. Null list and map
results do not become empty collections. A platform error preserves its
code, message and mixed integer/decimal details. A missing mandatory reply
uses the method and channel in its missing-plugin error; an optional
channel returns null. A success envelope carrying null is not mistaken
for a missing plugin.

The exact :65 and :101 nonempty collection registrations also ran. The
native typed list/map methods preserve their contents and independently
reject incorrect element types. These cases remain partial because the
raw Rust method returns an explicit JSON value rather than attempting
Dart's failing implicit generic List/Map runtime casts.

## Inbound handler replies

The exact :221, :237, :251, and :274 registrations passed. The native
receiver decodes the actual method bytes and accepts a one-shot,
asynchronously deferred envelope. An unimplemented method replies with
no bytes, success uses the one-value envelope, a specific platform error
retains its code and message, and a different typed application error
maps to code `error` with its display text. This adapts Dart exceptions to
Rust results, without catching Rust panics or replacing error types with
string matching.

Seven focused native channel tests and twenty nearby receiver, menu, and
optional-channel regression tests passed. The standard binary method
codec is unchanged. No credit is claimed for test messenger mock-handler
identity or for EventChannel subscription and cancellation.

## Basic request/reply messages

The exact pinned :14 and :24 `BasicMessageChannel` registrations passed.
The native sender puts unwrapped UTF-8 `hello` on channel `ch` and decodes
`hello world` from its reply. The receiver decodes those raw input bytes
and issues the same unwrapped UTF-8 reply in a later microtask, exactly
once. No method name or envelope is introduced.

Native controls also distinguish no bytes from an empty byte buffer
in both directions, reject invalid UTF-8, and round-trip nested JSON and
standard binary values through the same channel interface. The transport
hands the codec a nullable payload unchanged; older method callers wrap
their encoded request in `Some`. Three new native channel tests and 36
nearby platform tests passed; the browser platform crate was checked for
Wasm and the mounted editor integration was re-run.
