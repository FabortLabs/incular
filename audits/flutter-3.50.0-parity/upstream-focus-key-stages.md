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
