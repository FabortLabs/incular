# Focus navigation across irregular layouts and nested scopes

Pinned Flutter 3.50.0 `ab2912035e160d9cad6306df307576da6128ebdb`.
Reference: `packages/flutter/test/widgets/focus_traversal_test.dart`,
including two WidgetOrderTraversalPolicy and three
DirectionalFocusTraversalPolicyMixin registrations.

The native fixtures preserve the irregular dimensions, alignment, and
out-of-band gaps. All arrow transitions reach the same primary node.
At a scope edge no next target is returned and no widget receives a
duplicate focus-change callback. When the current band is empty, both
axes choose the next main-axis neighbor before cross-axis proximity.

Forward/backward node navigation uses the current primary descendant,
rather than starting over from the node on which the method was called.
Both nested scopes stay focused while their leaves cycle. FocusScope now
forwards the inherited onFocusChange callback for both supplied and
mount-owned nodes. It fires when a descendant enters or leaves the scope,
not when focus moves internally between two descendants.

Five exact pinned originals passed, as did three directional fixtures,
eight sequential fixtures (including an authored mounted callback case),
and adjacent 32 supplied-scope, four owned-scope, and five directional
history checks. Custom policy filtering is a separate open registration.
