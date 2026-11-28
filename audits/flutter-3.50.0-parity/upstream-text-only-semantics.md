# Text-only semantic properties

Pinned `widgets/basic_test.dart:740,788,801,814` passed. These put
an attributed current value, a plain decreased or increased value,
or an attributed hint on a Placeholder without a label. The native
runtime counterparts publish exactly one node with the specified text,
no invented label, and the enclosing direction in both LTR and RTL.
The current value and hint additionally preserve the typed UTF-16
pronunciation span in the right attribute field. All 11 focused
semantic string and locale tests passed.

Pinned attributed alternate value originals (:756 and :772) also
passed. Native runtime tests mount each value alone without a label
and verify inherited RTL direction and the matching typed UTF-16
span. With both values present, replacing one value with plain text
clears only its own spans; replacing both clears both without
recreating the node. These checks cover the retained semantics tree,
not pronunciation in a live screen reader. All 14 focused native
tests passed across the two semantic targets.
