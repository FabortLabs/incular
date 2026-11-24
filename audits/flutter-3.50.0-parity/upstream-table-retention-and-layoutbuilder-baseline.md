# Table row retention, accessibility and LayoutBuilder baseline

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

Five more original Table registrations passed on Flutter.
An empty table (:47) and repumping unchanged flex columns
with different labels (:357) match. Changing 3×3 to 4×2
(:274) preserves the first render identity on both. Rust's
flat slot 6 wrongly reuses the old third-row render for a new
second-row cell. Flutter treats the old row as removed.
An explicitly empty TableRow (:882) cannot be represented
with the flat Rust API; reference-only.

In the original structural accessibility case (:967), Flutter
groups the two named leaves as Table → Row → Cell. Incular's
real semantics tree contains both labels but only two Text
nodes, without table roles. This is partial against Flutter
host/direction wrappers, not a native assistive-technology
delivery check. See `flutter350_table_retention_semantics`.

The original LayoutBuilder case (baseline_test.dart:97) passes
on Flutter. A separately verified fixture compares actual text
directly and through LayoutBuilder and repeats both with a
no-baseline box. On Rust the direct text top is 98.86929
relative to the outer box, computed using the real reported
baseline 81.13069. With LayoutBuilder it becomes 80 although
the leaf retains its baseline. Both no-baseline controls
correctly use 80. The original custom RenderBox's reading is
not directly representable in this Rust test, so the
original is partial. See `flutter350_layoutbuilder_baseline_parity`
and `layoutbuilder_child_baseline_reference_test.dart`.
