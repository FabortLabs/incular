# Exact frame selection: overlapping gesture names

Added exact matching on both harness sides so a fixture can be checked
without silently running a different scenario with the same prefix. The
legacy Rust substring behavior remains available for other callers.
The focused audit runner always uses exact matching and refuses unknown
catalog IDs.

Both pinned 3.50.0 comparisons match in scratch batch
`20261124-155131-210`:

- `gestures__long_press`: 35 frames (the longer-named case is not included).
- `gestures__long_press_draggable_quick_move`: 5 frames.

No workspace suite ran.
