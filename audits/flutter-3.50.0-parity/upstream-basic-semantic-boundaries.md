# Basic semantic attributes and role boundaries

Five exact pinned `widgets/basic_test.dart` registrations passed:

- **342:** attributed label, value and hint retain their own ranges and
  annotation type, including en-MX on the value. The existing runtime
  integration test also verifies the mobile projection and that replacing
  metadata with plain strings clears the attributes in place.
- **412, 429, 489, 523:** nesting a text field, link, image or keyboard-key
  annotation under an explicit Dialog does not replace or absorb the
  original role. A separate child node retains the annotation.

Native: `flutter350_semantics_role_boundaries` and
`flutter350_attributed_semantics` (six passed). No production changes
were needed; other semantic roles and metadata are separate comparisons.

The pinned route-scope and slider cases at `:446,506,540` also
pass. The two originals titled "scopes route" are distinct:
`:446` applies scopesRoute with explicit child nodes; `:540` uses
the same slider arrangement as `:506`. The runtime checks both
physical node identities and the parent-to-child relationship while
preserving the child's independent flag. All 11 focused role tests
passed.

The Linux branch of `:468` also ran. Flutter's native runtime treats
a legacy header flag as a trait, allowing it to merge into the
Dialog; Flutter Web treats it as a distinct role. Incular's
heading projection is not yet verified against both branches;
this registration is not counted as paired.
