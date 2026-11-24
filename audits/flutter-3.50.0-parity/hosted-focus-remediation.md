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

## Ancestor focus membership

The runtime now records the retained path to primary focus. Parent
nodes expose has_focus while a descendant is primary, even if a parent
cannot take primary focus itself. Changes to the path are applied before
node observers run. Moving from parent to child or between siblings
no longer publishes a false end to the parent's focus membership.
The pinned authored switch test and the original partial ancestor
regression now pass, along with the hosted, keyboard, Visibility and
standalone controls.

## EditableText-owned focus node

The neutral EditableText can now attach an application focus node directly
and request initial autofocus. It shares the hosted eligibility and
attachment lifecycle instead of requiring an outer keyed event target.
The pinned and Incular probes verify programmatic focus, node replacement,
rejection of a later detached-node request, uninterrupted editing with the
same controller, and eligible/ineligible initial autofocus. The six focused
Rust targets pass 23 tests. This does not claim dynamic autofocus changes
or every platform's soft-keyboard connection behavior.
