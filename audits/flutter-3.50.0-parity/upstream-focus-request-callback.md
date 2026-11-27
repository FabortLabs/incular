# Custom traversal focus requests

The pinned WidgetOrder, ReadingOrder, and Ordered traversal cases at
`widgets/focus_traversal_test.dart:513,1140,1618` passed. Starting
from a fresh mounted scope, both Next and Previous dispatch the
selected live node to the group's callback. The callback can leave
focus unchanged without running the default focus or reveal.

Native controls verify that the initiating policy handles a
cross-group request and can request focus itself, and that spatial
traversal invokes the same override with the intended reveal edge.

## Scoped arrow traversal

Both exact directional originals (`:2405,3305`) passed on Linux.
An arrow request from a scope uses that scope's remembered child;
without one, it enters at the directional edge. All four arrows
call the mounted group's custom request on the chosen node,
without issuing default focus if the callback only observes.

The removal fixture queues focus on the top control, takes two
Down requests to reach the bottom, then removes the middle
control. Bottom still has primary focus; Up skips the stale
history stop and reaches the live top node. This uses an unkeyed
retained rebuild with the same external focus handles. Sixteen
focused native tests passed including irregular spatial layouts,
existing history, and the scroll-to-reveal controls.
