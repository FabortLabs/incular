# Flutter-compatible native spelling channel

The authored Flutter probe installs a mock standard method channel and
runs DefaultSpellCheckService. It sends `SpellCheck.initiateSpellCheck`
on `flutter/spellcheck`, with the BCP-47 locale followed by the checked
text, and preserves the returned order of replacement strings. An empty
result is success; a busy response returns no result and keeps the prior
successful snapshot.

Incular's opt-in host transport tests exercise the same method envelope
and asynchronous response. Flutter's offsets 3..7 after an emoji map to
Incular bytes 5..9. Additional Rust controls cover a missing response,
a null reply, a UTF-16 surrogate split, and out-of-bounds or reversed
spans. Incular declines such a reply rather than publishing a partial
answer. This does not attach a device's native dictionary automatically,
and authored controls do not change the upstream registration count.
