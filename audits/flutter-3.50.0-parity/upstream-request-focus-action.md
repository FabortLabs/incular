# Explicit focus and reveal action

Pinned `widgets/focus_traversal_test.dart:3935` passes. A mounted
default `RequestFocusAction` focuses its target after the next frame.
After unfocus, an intent callback is called exactly once with the same
node and owns the operation: no implicit focus or scrolling occurs.

The authored Flutter probe invokes the inherited WidgetsApp action.
The 60px target ends at content y=960 in a 600px viewport. Even when
already visible at scroll position 450, invoking the action moves to
360, aligning the target's trailing edge. This distinguishes explicit
alignment from keeping an already visible target where it is.
The native test also checks a nearer action overrides the default and
that a disabled override does not fall back to automatic focusing.

Incular's `Actions` maps command values, so a local override is registered
for a particular `FocusNode`; Flutter's action table is keyed by intent
type. Callback identity is not part of the Rust command key. Native
explicit calls and shortcuts run the WidgetsApp default, but that
built-in is not returned by the generic action-handler lookup API.
These separate action lookup differences are not counted as verified.

Three native action tests and 22 adjacent focus, action-dispatch, and WidgetsApp checks passed.
