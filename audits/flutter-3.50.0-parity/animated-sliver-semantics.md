# Animated sliver opacity: accessibility independent of paint

The pinned Flutter 3.50.0 explicit and implicit sliver opacity constructors
default `alwaysIncludeSemantics` to false. Their shared render mixin includes
children if the quantized alpha is nonzero or the flag is true. A flag change
invalidates semantics independently from paint.

An authored Flutter test traverses the actual published semantics tree for
both widgets, verifying that at zero opacity the default does not publish the
label, opting in publishes it without enabling paint, and opting out removes
it without a remount. Bringing opacity to 0.5 restores paint and the label.
Both pinned cases passed. Looking up the original widget by its semantics
label is not sufficient: the widget can still be found after its node has
been removed from the published semantics tree.

Incular now passes the flag through its retained sliver opacity scope and
its box fallback, including the direct-controller extension. The animation-
backed box transition can also keep semantics at zero. The native
`flutter350_sliver_opacity_layers` and `flutter350_render_opacity` targets
passed 17 tests, including mounted paint, semantics, and identity assertions.
These are authored comparisons and add no original-registration credit.
