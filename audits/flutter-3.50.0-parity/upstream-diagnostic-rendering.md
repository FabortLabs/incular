# Foundation diagnostic rendering

## Iterable properties

Flutter 3.50.0's original `foundation/diagnostics_test.dart:1511` passed.
The focused native tests in `flutter350_diagnostic_tree` verify typed integer,
double, and mixed-object values; absent versus empty collections; default-level
filtering; and string-valued inspector members distinct from display rounding.
An iterable's own whitespace style renders each member on a separate line,
but a compact parent renders the same members in brackets on one line to
avoid ambiguity with sibling properties. A one-member iterable stays on one
line even in the whitespace style. Tests compare complete output, including
prefixes, spacing, punctuation, and newlines. The tree, diagnostic properties,
and messages/flags targets passed all 32 focused native tests.
