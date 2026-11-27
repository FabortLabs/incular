# Focus keyboard handler stages

Pinned Flutter commit: `ab2912035e160d9cad6306df307576da6128ebdb`.

The two `widgets/focus_manager_test.dart` registrations :2107 and :2186
passed under each of their exact RawKeyEvent and ui.KeyData/RawKeyEvent
variants. They remain partial matches: Incular routes one modern
`KeyboardEvent`; it does not implement the deprecated parallel raw-key
payload and callback count covered by those variants.

The authored `focus_key_stages_test.dart` and native
`flutter350_focus_key_stages.rs` compare ordered modern dispatch and
platform consumption for key-down and key-up with explicitly focused
nodes. An early `Handled` skips the focused path and late callbacks.
A focused node's `Handled` prevents the late callbacks. An early
`SkipRemainingHandlers` falls through to the focus path in this pinned
reference; the same result at the node stops that ancestor walk, but
still allows a late handler to handle the key. If the late handler also
skips, the native event is unconsumed. A second native test shows that
removing a later handler while a stage is dispatching changes the next
event, not the current callback snapshot.

The new stages are specific to each runtime, and are independent of the
hardware handler list. That list still visits every registration and
reports alongside focus dispatch. Root-scope routing without a mounted
focus target and deprecated raw-key transport remain unproven.

## Window key stages with only implicit root focus

The authored pinned Flutter and retained runtime tests both start with
no focused widget. A handled early key prevents the late stage; ignored
early allows late to handle it; late SkipRemainingHandlers leaves the
key unconsumed. Incular initially failed the first check because both
stages required a focused element. They now run for a mounted root
without inventing a focused child. The focused-node and registration
snapshot controls still pass (ten targeted Rust tests).


## Raw-only and combined key messages

Both :2107 and :2186 now pair at the keyboard message boundary.
The pinned Linux reference passed all four expanded Flutter variants.
The retained runtime test replays each sequence through two distinct
portable ingress modes: a raw key that supplies the typed projection,
and a combined typed/raw message. Both update hardware keyboard state
once; the Focus receives typed then raw once each unless an early
handler consumes that key. Late registration, consumption, removal,
and key-up are checked separately. A typed-only message does not
fabricate a raw event, and a combined message preserves the two
supplied key views even when their logical labels differ.

Twelve targeted Rust tests passed. This does not emulate native
scan tables or the Flutter engine/test channel itself; the paired
behavior here is framework focus routing, not OS keyboard delivery.

### Focused ancestry consumes raw keys once

The original `widgets/focus_manager_test.dart:1406` passed in both
`RawKeyEvent` and `ui.KeyData then RawKeyEvent` modes. The native retained
fixture dispatches raw-only and paired messages through the runtime, using
two mounted scopes, two parents and four leaves. Their authored focus
parent links are separate from widget ancestry. The focused leaf sees
MetaLeft first. If it ignores the key, the parent sees it, then the
scope; if any handles it, delivery stops there and the native event is
consumed. Ignoring the key at every eligible node leaves it unconsumed.
After focus moves to the opposite scope, neither the former scope nor
unfocused siblings receive the event. The exact path checks also reject
duplicate deliveries on the paired route.
