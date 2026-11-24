# Focused frame rechecks, batch eleven

Fresh strict frames: Flutter 3.50.0 `ab2912035e` and Incular
`c647cb85`. Cumulative: **60/253 existing registrations (23.72%),
54 match, six differ**.

| Scenario | Result | Frames |
| --- | --- | ---: |
| `gestures__scale_single_pointer_pan` | match | 6 |
| `layout__table__rows_and_columns` | match | 2 |
| `paint__data__edge_insets_lerp_extrapolated` | match | 3 |
| `semantics__enabled_default` | mismatch | 2 |
| `text__editable_initial_selection_focus` | match | 3 |

The neutral semantics fixture sets a button role and tap callback but
does not set `enabled`. At frame 1 (16 ms), Flutter reports no explicit
enabled state (fixture value 2) while Incular reports enabled (value 1).
The tap action and other captured fields matched. Flutter's
`SemanticsConfiguration.isEnabled` can carry an unset state
(`semantics/semantics.dart:6408–6410`); the Incular tree fixture reads
a Boolean. This is a semantics representation difference, not proof
of an announcement or focus difference on a native screen reader.
Scratch batch: `20261124-153110-911`; [focused runner](run-focused-frames.ps1).
