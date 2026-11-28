# Text-only semantic properties

Pinned `widgets/basic_test.dart:740,788,801,814` passed. These put
an attributed current value, a plain decreased or increased value,
or an attributed hint on a Placeholder without a label. The native
runtime counterparts publish exactly one node with the specified text,
no invented label, and the enclosing direction in both LTR and RTL.
The current value and hint additionally preserve the typed UTF-16
pronunciation span in the right attribute field. All 11 focused
semantic string and locale tests passed.

Attributed alternate values (:756 and :772) are separate from these
cases: retaining their plain strings alone would not verify their
pronunciation attributes.
