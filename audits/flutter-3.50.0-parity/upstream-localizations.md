# Flutter 3.50 neutral Localizations

## Ancestor lookup and zero-size layout

The pinned `widgets/localizations_test.dart:82,100,214` registrations
passed. The Rust environment target checks the strict ancestor lookup
fails with the intended diagnostic, the optional lookup returns no
locale outside the scope, and an actual centered zero-size scope has
zero geometry without exposing its child to a missing locale at build
or paint time. Descendants also inherit directionality from the resolved
locale, as Flutter's Localizations supplies through its widgets delegate.

The Rust strict accessor panics rather than raising a Dart assertion.
The loader and dispatcher tests are not covered by these comparisons.

## Local pronunciation scopes

Both pinned Flutter `Semantics` registrations passed. As in
`widgets/localizations_test.dart:157`, the labels `1` and `2` inherit
Faroese from the outer Localizations, while `3` inherits Chinese from
the nested one. The Rust test also updates the outer locale to Spanish
and checks the labels retain their platform-independent identities.
As in :189, application-level Localizations exposes the locale to
builders but does not set per-node pronunciation overrides.

The AccessKit test additionally verifies that a native node keeps its
identity when its pronunciation language changes or clears. The
application-wide engine/dispatcher notification and mobile native
pronunciation injection are not part of this comparison.
