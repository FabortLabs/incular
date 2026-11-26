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

## Compatible hints and values

The retained ports of :331 and :398 now pair. Two hints form one
newline-separated hint. A hint and one compatible value live on the
same semantic container. The container takes the fragments' reading
direction when it has no text of its own. Two conflicting nonempty
values remain independent nodes, including their own hints. Promoting
and removing a conflict restores the merged owner without replacing
an unrelated explicit boundary. Geometry-only passes reuse whether
the owner already holds an absorbed value.

The two pinned authored probes also check UTF-16 attribute ranges
through an emoji, a separator and a direction mark. The RTL hint
span shifts to 4..6; the independent value span stays at 0..4. Those
probes and all 39 focused Rust tests pass. Strict frame rechecks:
`semantics__value_hint_tooltip` (2), `semantics__increase_decrease` (3),
and `semantics__merge` (2) all match. This does not implement the full
Flutter semantic configuration conflict reducer; other annotations
keep their independent roles, fields and actions.
