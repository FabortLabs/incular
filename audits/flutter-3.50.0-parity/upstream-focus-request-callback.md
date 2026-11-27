# Custom traversal focus requests

The pinned WidgetOrder, ReadingOrder, and Ordered traversal cases at
`widgets/focus_traversal_test.dart:513,1140,1618` passed. Starting
from a fresh mounted scope, both Next and Previous dispatch the
selected live node to the group's callback. The callback can leave
focus unchanged without running the default focus or reveal.

Native controls verify that the initiating policy handles a
cross-group request and can request focus itself, and that spatial
traversal invokes the same override with the intended reveal edge.
The separate original for scope-directed arrows remains unpaired.
