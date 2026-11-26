# EditableText spelling and localization

An authored Flutter 3.50.0 probe compares actual service calls from
EditableText: explicit Turkish wins over surrounding British English;
removing the explicit locale makes the editor inherit British English;
changing the existing Localizations scope to Canadian French affects
the next edit without replacing the editor. German keyboard language
hints remain independent. The native Incular focused runtime regression
matches the exact text and locale of each call and keeps the same focus.

The editor's explicit or inherited paragraph locale is also passed to the
Incular shaping style when the span style has no more specific locale.
No locale-sensitive glyph image is claimed by the service-call test.
Without any localization, Incular still falls back to its runtime locale
or the undetermined locale; the bare Flutter widget reports an error in
that case and does not request suggestions. This authored comparison is
not a new upstream registration.
