# Table direction, row validation, and intrinsic widths

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

Ten exact Table registrations passed on the pinned reference.
All rows have correct 100/110/125 fixed widths and Table width
335. LTR relative left coordinates match [0,100,210] (:136).
In ambient RTL (:194), Flutter uses [235,125,0]; Incular still
uses the LTR coordinates for every row. This is direction
handling, not a text width or operator precision issue.

Rust accepts an incomplete final row in its flat row-major
API. The original one-cell/empty-row test (:800) is partial.
The pinned authored fixture uses rows of width two and one,
which can be expressed as the exact same three cells in Rust,
and proves Flutter rejects those unequal rows. A complete
two-by-two Rust control succeeds. See
`flutter350_table_direction_and_rows` and
`table_incomplete_row_reference_test.dart`.

The explicit TableBorder smoke test (:252) and the missing
textBaseline default assertion (:787) are reference-only:
Rust lacks TableBorder and baseline-aligned table cells.
No surrogate clip or synthetic border was treated as parity.

Six/seven flex column small-deficit layouts (:310,:327)
pass without errors. Intrinsic sizing, resizing all cells
of a previously wide column, and switching from flex to
intrinsic (:391,:415,:452) match the original relative size
and equal-height checks. This is not a claim about identical
glyph widths between engines. See
`flutter350_table_intrinsics_parity`.
