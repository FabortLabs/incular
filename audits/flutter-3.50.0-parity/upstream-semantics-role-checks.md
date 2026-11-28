# Debug validation of emitted accessibility roles

The 15 selected originals in Flutter 3.50.0
`widgets/semantics_role_checks_test.dart` are paired against the actual
retained semantic forest, not layout-widget names. The native test
mounts and lays out the same shape, runs semantics assembly, and
checks the diagnostic on the node actually emitted.

| Source lines | Behavior |
| --- | --- |
| 16, 29, 42 | A tab must declare selected (false counts); an enabled, unblocked tab needs tap. |
| 59, 75 | A list item needs a semantic List parent; intermediate layout-only widgets do not count. |
| 91, 107, 124 | A tab bar must have children, and each child must be a tab. |
| 320, 336, 394, 410 | A menu needs content, and a menu item needs a menu ancestor, not necessarily a direct parent. |
| 594, 614, 634 | Alert and Status already communicate their role; explicit live-region duplication is invalid. |

Incular's framework checks run in debug builds after semantics
assembly. They expose a typed report carrying the retained node ID
instead of throwing a Dart framework exception. The release bridge
does not incur automatic diagnostic traversal. Failure and success
conditions were compared; exception/diagnostic presentation is
language-specific. MenuBar, RadioGroup, and landmark roles are not
credited here.

The initial native run showed that annotating ExcludeSemantics
removed the outer annotation as well as the child. It also showed
that applying explicit-child semantics directly onto a Text child
collapsed the two semantic nodes. Both now retain a distinct
transparent annotation boundary when necessary. The authored pinned
tab test verifies disabled and action-blocked exceptions, and the
native suite verifies correction clears an old diagnosis.
Six role tests, 24 adjacent semantics regressions and five root
regressions passed.

## Slider and progress range data

The 20 originals at lines 1082–1428 compare missing fields,
malformed values and bounds, inverted bounds, ordinary values
outside the declared range, and percentage values outside 0–100%.
Valid percentages use their own scale even when the declared numeric
bounds are 0–10. The tests also verify permissive valid values on
the boundaries. Textual min/max annotations retain their declaration
and publish finite typed limits for existing native bridges. Incular's
typed numeric API continues to validate without requiring duplicate
strings. Non-finite values are not published as numeric bounds.

All 20 selected originals and 33 targeted native tests passed.
The typed report's language-specific diagnostics are not compared
as string snapshots.

## Menu bars and checkable menu items

The 11 paired originals at 357, 373, 428, 448, 464, 481, 500, 521,
537, 554, and 573 verify that menu bars cannot be empty, and that
ordinary menu items accept either a menu or a menu bar ancestor.
Checkbox and radio menu items require a real declared checked state
(explicit false counts) before their ancestry is checked. A native
AccessKit projection regression verifies distinct roles and toggled
flags. Browser ARIA mappings were added, but browser execution is
not claimed by this native test slice.

All 11 selected originals, 11 focused native widget tests, and 6
AccessKit flag tests passed.

## Radio group boundaries and checked-state rules

Nine pinned originals (145, 164, 194, 221, 254, 270, 294, 652, 685)
are paired. Only checked descendants marked mutually exclusive
count toward one group; a nested RadioGroup owns its own set. Labels,
non-radio checks, unchecked radios and toggled-but-not-checkable
children do not cause a false duplicate. The two additional
registrations in the alert/status group repeat the labelled-radio and
non-checkable shapes and both ran explicitly.

The native runtime probe also mounts real raw controls in a typed
controlled group. Pointer selection updates both rendered and semantic
state, Space clears a toggleable selected radio, a disabled radio
does not activate, and the focused radio keeps its node through the
controlled value update. The first native run exposed stale checked
state because the group lookup was nonreactive; the read now binds
the dependent raw radio to the mounted inherited registry.

Twelve targeted widget tests and one targeted runtime test passed.
Arrow wrap, tab-entry policy, and platform-specific accessibility hints
are separate coverage.

## Landmarks

All 17 `landmarks` registrations (703–1070) ran against the pinned SDK
and mounted native semantics. Complementary, content information and
main landmarks cannot be inside another landmark. Repeated
complementary/content information/main/navigation roles require a
nonempty label on each instance, across independent retained
parents. The pinned checker does not compare equal nonempty labels.
A standalone navigation landmark needs no label, but even a single
region does.

AccessKit projection maps the five roles separately. Equivalent ARIA
mappings are present without claiming browser execution. Fifteen
focused role tests, five semantic-root regressions and seven AccessKit
flag/projection tests passed.
