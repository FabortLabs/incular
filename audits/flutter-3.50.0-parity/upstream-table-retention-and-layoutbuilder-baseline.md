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

## LayoutBuilder baseline repair

LayoutBuilder now forwards the generated child's actual baseline and
clears the value when the child has none. Flutter's original at line 97
was rerun; the focused Incular port uses a real Baseline widget to report
the same 20 px metric in place of Flutter's test-only render detector.
Both place the generated child at 160 px relative to the surrounding
baseline's origin. Additional tests verify text metrics, the bottom-edge
fallback, and replacement of the retained generated child across
20 px → absent → 30 px. All four LayoutBuilder and both Stack baseline
checks pass.

## Row-aware column-count changes

The pinned `:274` original and native port now agree. Reconciliation
matches each row separately instead of letting old flat indices decide
which cells survive. An authored comparison also checks all surviving
cell identities in 3×3 → 4×2 → 2×4 transitions and confirms that newly
inserted columns and rows do not inherit state from removed cells.
The flat Table API still permits a short final row; explicit Flutter
TableRow keys, empty rows, and table accessibility roles are separate.

## Retained table accessibility

Three named Flutter originals pass: `:967` exposes Table → Row →
labeled Cell; `:1026` tolerates a cell descendant painting past the
row bottom; `:1068` keeps the same cell wrapper when the real editor
takes focus. The native ports exercise these same assertions. The
native graph tests also verify that zero-width cells and removed or
disabled tables leave no unreachable semantic nodes. An authored
Flutter comparison and native port move explicit cells from LTR to RTL
and preserve the row and cell node IDs.

The AccessKit consumer independently validates the table, row, cell,
and column-header role mapping. This does not assert native reader
announcements, synthetic row scroll actions, or the Flutter shell's
generic wrapper nodes.

## Row reveal after a retained layout change

An authored Flutter probe confirms an upstream cache issue: in a 120px
viewport, growing the first row from 50px to 100px moves the second
40px row down by 50px and preserves its semantic ID. Its retained
ShowOnScreen action leaves offset 0. A fresh Table at the same geometry
reveals the second row and reaches offset 20. Flutter registered the
closure against the first row rectangle.

Incular deliberately reveals the **current** bounds, reaching offset
20 for both the retained and fresh Table. The runtime test verifies
that the old synthetic ID cannot dispatch after removal. Existing
PageView reveal tests still pass. This authored comparison does not
alter the pinned original coverage count.

## RenderTable sizes and flat-cell updates

Pinned `rendering/table_test.dart:43,50,161,174,295` passed by exact name.
The native retained table shrinks to zero under loose constraints, divides
100 pixels among six actual cells, and detaches a removed last-row cell
before another layout. Two updates move the same three keyed cells
across the six slots without changing the table's own identity or its
two-row, three-column geometry. The tight 800×600 RTL fixture also uses
the original seven flex weights: 1 and six copies of 0.123. The measured
cells remain finite, retain their 100-pixel height, and meet edge to edge
without losing width. All five focused checks passed.
