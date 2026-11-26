# Clipboard channel reply boundaries

All five pinned `services/clipboard_test.dart` registrations passed.
The two previously open registrations (34, 40) are now paired: a
null reply means no clipboard data, but a returned object without
a text field or with null text is invalid. Rust exposes typed errors
instead of Dart's TypeError. The actual JSON method envelopes are
checked; `Clipboard.getData` sends `text/plain` and
`Clipboard.setData` sends the text object. Additional native controls
check empty strings, wrong field types, and boolean string presence.

This is the optional proof of behavior at the embedder's channel
boundary. It does not replace the multi-format native Clipboard API
or read the user machine's real clipboard.
