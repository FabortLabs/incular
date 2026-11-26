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
