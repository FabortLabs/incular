# Flutter text-input configuration

Four pinned `services/text_input_test.dart` registrations pair
(:559, :572, :607, :627). The typed Flutter wire configuration
has a plain text keyboard, mutable and unobscured editor, enabled
autocorrection, no custom action label or inline-prediction setting,
no delta opt-in, light appearance and no automatic capitalization.

Plain text has null numeric flags. A decimal keyboard preserves the
decimal flag even when editor content is obscured. A numeric password
keyboard sets the independent password hint without setting the
editor's obscure flag. Resolved smart quote and dash policy,
localized hints and the unset-versus-false inline prediction field
continue to serialize consistently.

Autofill now uses the same serialization path instead of repeating
its key names. The typed delta attachment writes the same native
configuration and forces only its own delta flag, without mutating
the caller's configuration. This compares Flutter's JSON channel,
not the appearance of a host platform's keyboard. Four original
Flutter and 13 focused native tests passed.

## Equality, copy, and hashing

Exact pinned `services/text_input_test.dart` registrations :346 (equality),
:413 (copyWith), and :482 (hashCode) all passed. The native
`flutter350_text_input_config_identity` target uses the original configuration
shape: view 1, action label "label1", enabled smart dashes and quotes, and
ordered text/PDF MIME types. It checks a no-op copy, field and wire values,
equal hashes, and use as an equivalent map key.

The additional pinned probe `text_input_copy_effective_defaults_test.dart`
confirmed that explicitly enabled smart punctuation equals and hashes like the
implicit default for visible text. Changing only obscuring through `copyWith`
keeps the source's effective punctuation; constructing an obscured
configuration from scratch instead defaults both options to disabled.
Before the fix Incular emitted identical JSON for implicit and explicit
settings, but the two configurations compared unequal. Both focused native
configuration targets passed after the fix (10 tests total).

Rust compares owned locale lists by ordered values instead of Dart list
identity. Its hash number is not the Dart hash number. The Rust copy callback
also permits explicitly resetting either option to the new obscuring default
with `None`, something Dart's optional null arguments do not express.
