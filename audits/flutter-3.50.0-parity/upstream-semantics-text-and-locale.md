# Semantics text direction and nested locales

Pinned Flutter 3.50.0 `ab2912035e`; eleven exact originals passed.
Nine pair with retained Incular observations here.

- At `widgets/semantics_test.dart:188`, locale-only annotations form
  two nested nodes with independently retained language tags. Incular
  previously dropped both annotations. The control updates the inner
  locale without changing the two IDs or adding an implied text direction.
- At :225, :239, :253 and :278, a label follows either ambient direction,
  and an explicit semantic direction takes precedence in both directions.
- At :100 and :303, the tooltip and the combined label, hint and value
  preserve their fields and direction. At :362, two values remain
  separate children and each gets ambient direction, including after a
  direction-only update. At :1199, the current value and both alternate
  values share a node with both executable adjustment actions.
- A pinned authored RTL control shows that hint, tooltip, increased
  value and decreased value each need a reading direction even without
  a label or current value. Incular now supplies it without inventing
  label text. Empty locale-only nodes still leave direction unspecified.

The :331 and :398 references are verified but not yet paired. Merging
two hints, or a hint and one non-conflicting value, requires separate
retained compatible annotation absorption. The two conflicting values
above must remain separate after that work. Twenty-six focused Rust
tests and the authored Flutter control passed.
