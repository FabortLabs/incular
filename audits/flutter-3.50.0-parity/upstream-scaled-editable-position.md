# Scaled EditableText positions after scrolling

The original Flutter 3.50.0 non-web registrations in
`widgets/editable_text_test.dart:18281,18321` passed. The native test mounts
the same eight lines in a centered editor at scale 0.5, with exactly two
visible lines and no initial selection. The top-left point starts at
offset 0. Explicitly revealing position 18 does not focus the field or
set the selection; it brings the third line to the top at offset 12.
Selecting across 100 world pixels then gives the exact offsets 12..17.

The shared position reveal uses the editor's actual inner scroll and then
projects the updated caret into enclosing viewports. An unset selection no
longer automatically reveals the controller's fallback caret at the end.
Hit testing converts through the retained transform rather than treating
global coordinates as editor coordinates.

## Word boundaries and affinity

The pinned scaled whole-word selection and word-edge originals both
passed. Incular previously looked up the byte after an upstream caret
when selecting its word. Dragging past the visible line end therefore
included the hard newline at byte 17 (extent 18). The word boundary now
uses the previous character for upstream affinity, preserving the exact
12..17 range. The separate word-edge operation at byte 13 collapses at
17 with upstream affinity. The same position-based selection code serves
the retained double-tap drag. Four scaled controls and 21 adjacent
double-tap and editing-shortcut tests passed.
