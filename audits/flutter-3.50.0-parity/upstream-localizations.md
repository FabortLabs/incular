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
