# WidgetsApp navigation focus scope

Pinned `widgets/app_test.dart:465,496,527,555` passed. The native
navigator-style app and both Router configurations mount a retained
scope named `Navigator Scope`. The app builder receives the scope
itself, not the routing child directly. The mounted route's focus
node resolves that scope as its enclosing boundary.

All four earlier native fixtures failed because the app did not
establish a navigation scope. On both paths, the scope now receives
the fallback autofocus request. It does not advance into an
unrequested leaf, while an explicit descendant autofocus request
still wins. The authored pinned
`widgets_app_navigation_autofocus_test.dart` passed both modes.
The existing explicit first-descendant autofocus option on Incular's
FocusScope is not repurposed. Four focused and 35 adjacent runtime
checks passed. Route/overlay semantics nesting is a separate audit.

## Route and overlay semantics remain separate

Pinned `widgets/app_test.dart:585,637,666` passed. A focus scope
keeps explicit semantic children without exposing a keyboard-focus
action of its own. The named-route controller also marks its route
as a route scope instead of letting it merge with the application
scope. Disabling the focus-scope annotation leaves the mounted
leaf's real keyboard focus and accessibility label intact.

The native Router fixtures mount a real Navigator and retained
RouteOutlet. Inserting a nonmodal overlay entry keeps the original
route's semantic parent and the outer navigation scope at their
previous IDs; the overlay is a separate direct child of the
navigation scope. The delegate configurations and the named-route
controller all preserve the two-level semantic boundary. Four
direct and 59 adjacent focus/shortcut checks passed.

## Builder-only defaults and zero-area layout

Pinned `widgets/app_test.dart:29,694,1008` passed. A builder-only
app mounts its keyed root and builder content, and the same app
under a shrink box lays out at exactly zero size without crashing.
The new `ScrollConfiguration::of` reads the mounted builder's
inherited behavior, falling back to the standard behavior when no
ancestor provides it. An additional native control changes the
nearest configuration from touch to mouse and verifies the mounted
builder is notified. Four focused and eight adjacent tests passed.

## Default and custom app activation

Pinned `widgets/app_test.dart:43,68,101` ran successfully (including
both simulator variants of :101). The mounted builder-only app now
binds unmodified Space to the standard `ActivateIntent` and resolves
the checkbox analogue's focus-local action. Supplying an app shortcut
and action for the same key replaces the default: the custom action
fires once, while the checkbox value stays unchanged. Releases do
not invoke either mapping.

Incular also verifies native main Enter, keypad Enter, and Select
against the same mounted control, and that Control chords do not
trigger it. :101 is partial because the portable event path does not
carry Flutter's GameButtonA or replicate its simulator transports.
No full application key table or platform gamepad support is
claimed. Three focused and 17 adjacent Rust tests passed.
