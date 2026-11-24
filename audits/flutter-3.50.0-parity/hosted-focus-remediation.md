# Hosted focus requests

Flutter 3.50.0 does not make a node focused merely because it requested
focus. The runtime now applies the latest eligible request at its next
input or frame boundary. Requests denied while a mounted branch is hidden
or ancestor-blocked cannot change the public primary-focus value, trigger
a false focus callback, or replay after selection. Unmount clears focus;
a later request made while detached waits for reattachment. Explicit
requests can select a node that skips traversal. Standalone WidgetTree and
FocusManager users retain their existing direct APIs.

All three pinned `hosted_focus_requests_test.dart` probe cases and the
matching Rust target pass. Both IndexedStack focus cases and its key
routing, default Visibility replacement, and cross-fade focus pass.
The existing standalone widget and gesture controls also pass (20 and 5).

Not changed in this patch: ancestor `has_focus()` membership, Visibility's
independent maintain-focusability option, the deliberate invalid-index
handling, and the earlier canceled-touch/editable focus regression (B01).
The named audit targets still report those known reds rather than
suppressing them.
