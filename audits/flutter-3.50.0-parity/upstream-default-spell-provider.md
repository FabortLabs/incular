# Default platform spelling provider

The authored Flutter probe enables its platform checker capability and
wires a mock native spell-check channel. An editor with a style but no
explicit checker sends its user edit to that channel. An explicitly
provided checker still wins, and an obscured field sends nothing.

Incular's mounted test uses an embedder-supplied factory. Two opted-in
editors have separate default checker instances, a separate custom
checker is untouched, and the password editor never constructs a
provider. Removing the capability clears rendered spell styling and
rejects an outstanding result from the old session. No particular OS
dictionary is installed by this API.

The pinned no-provider registration (editable_text_test.dart:16448)
also ran. Incular matches the disabled runtime state when no default
checker is available, but does not emit Flutter's captured debug
framework error; that registration is partial.
