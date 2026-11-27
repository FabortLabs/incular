# Selection bounds and directional drag offsets

Both pinned rendering originals pair. The rectangle classifier
checks all eight outside regions and the center. Drag offsets inside
the rectangle stay unchanged; outside points snap to the top
leading or bottom trailing corner in either writing direction.

A separate exact-edge probe covers the subtle difference between
classification and drag adjustment. Flutter excludes right and
bottom edges from the inside area. At the top-right corner the
classifier returns next but the drag still snaps to the first
corner; at the middle of the bottom edge the classifier returns
previous but the drag snaps to the last corner. The implementation
does not change Incular's general-purpose hit-test rectangles.
This slice checks the rectangle policy independently of mounted
selection gestures.
