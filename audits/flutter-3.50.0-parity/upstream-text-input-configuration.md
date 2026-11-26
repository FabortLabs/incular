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
