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

## Error message wrapping

The pinned original at `foundation/diagnostics_test.dart:1674` passed.
Incular reproduces the complete output at 40 columns in all three cases
from the original: single-line properties inside a framed error, flat
properties in a sparse tree, and error properties in a framed error. Tests
compare the exact borders, wrapping spaces, indentation, and final newlines.
URLs and other unbroken tokens may exceed the width without being split;
explicit line breaks and matrix rows survive, and word wrapping can be
disabled on a description independently of its label. The Unicode regression
checks width in UTF-16 columns, as on the reference. All 34 focused native
diagnostics tests passed.
