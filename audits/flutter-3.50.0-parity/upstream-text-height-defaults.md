# Inherited text height behavior

Three pinned `widgets/default_text_height_behavior_test.dart` registrations
are paired. The retained paragraph now uses Flutter's precedence: explicit
`Text`, nearest `DefaultTextStyle`, then `DefaultTextHeightBehavior` and the
built-in default. Either order of nested scope wrappers gives the same
precedence. An unset inner style leaves the separate ambient scope in
control, not a more distant style.

The original propagation test accidentally gives both providers equal
values. A supplemental Flutter probe verifies that equal providers do not
rebuild a subscribed child, while a real change rebuilds once and updates
the laid-out line height. Native tests verify matching resolved values,
measured heights, and retention of the existing text element.
`AnimatedDefaultTextStyle` also applies the new height behavior immediately
on its update frame, while letter spacing still interpolates.

The no-provider case verifies that the public lookup returns none and the
native paragraph uses default metrics. Flutter's generated `RichText`
stores a nullable field, whereas the native render node stores a resolved
concrete default, so this registration is partial. Four pinned Flutter,
three supplemental Flutter, and six named native cases passed.
