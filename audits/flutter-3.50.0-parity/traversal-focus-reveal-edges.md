# Traversal reveals the side of the newly focused control

Both pinned Flutter 3.50.0 probes passed with three 70px focus stops in
a 120px scroll viewport. The next two Tab or Down requests reveal trailing
edges at offsets 20 and 90; reverse Tab or Up reveals the previous leading
edges at offsets 70 and 0.

Previously Incular changed the keyboard target but left the viewport at 0
in both modes. The runtime now uses the retained Scrollable ensure-visible
path when traversal succeeds (including directional-history reversals).
It uses KeepVisibleAtEnd for forward Tab, Down and Right, and
KeepVisibleAtStart for reverse Tab, Up and Left. Programmatic or pointer
focus changes are not sent through this path.

Both native reveal tests passed after the fix. All 14 focused native
tests across reveal, focus paths, edge policies and caret scrolling passed.
This authored comparison does not add original registration credit and
does not expand Scrollable's existing coverage of two-dimensional or wheel
viewports.
