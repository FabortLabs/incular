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

## Named-route fallback delivery

The authored `widgets_app_unknown_route_delivery_test.dart` probe
matches mounted behavior: the generator declines an unlisted route
once, the unknown-route callback supplies its page once, and Back
restores home without repeating either callback. The native baseline
accepted the push but never displayed the fallback. Resolved
generated widgets now belong to the admitted stack entry.

Two additional native controls verify that refusing an unlisted
route leaves the current page intact, and that a deep-link
notification can read the newly committed route without borrowing
the stack while notifying observers. The three focused native
controls and 36 app-shell checks passed. This does not claim parity
for Flutter's diagnostic wording or add original registrations.

## Caller-generated startup stack

Pinned `widgets/app_test.dart:230` passed. The custom startup
factory receives the original `/abc` path once, supplies two pages
outside the normal named generator, and presents the second page.
Back reveals the first without displaying the regular page or
running the startup factory again. Duplicate route information
does not collapse distinct startup entries.

Native controls also verify that later platform deep links use
the named generator, and that restoring a surviving startup
prefix preserves its page by position, even when two pages
share a route name. Three direct and 39 adjacent checks passed.

## Platform Back reaches the root router

Pinned `widgets/app_test.dart:286,322,409` passed. System Back now
reaches the real mounted router delegate with a provider/parser,
with only a delegate, and with an explicitly supplied root
dispatcher. Each fixture starts with the visible `initial` route,
delivers one system Back, and then exposes only `popped`.

The direct WidgetsApp router constructor supplies a platform root
dispatcher. An explicit RouterConfig retains its own choice. The
root's platform registration now follows its callbacks even when
the router holds it through the shared dispatcher type; a plain
child dispatcher is not inadvertently registered as a platform
root. Three focused and 44 adjacent checks passed.

## Route information defaults and invalid configurations

Pinned `widgets/app_test.dart:352,446` passed. A direct router with a
parser and no supplied provider receives the ordinary startup
location `/`, which appears in its mounted content. Rust's fallible
configuration constructor rejects a provider with no parser before
mounting, corresponding to Flutter's constructor assertion. The
two direct native checks and seven adjacent routing checks passed.

## Initial platform route and named-prefix order

Four authored probes passed against the pinned Flutter checkout. A
non-root platform launch route takes precedence over the configured
initial route; a platform root uses the configured route. Default
startup asks the generator for root, intermediate prefixes, and leaf
in that order. If the leaf is unknown, it discards the partial stack
and asks for root again.

Three native tests cover those four scenarios, including the visible
page and exact generator trace. All three and 41 adjacent app and
routing tests passed after the fix. These authored cases do not add
to the original upstream case count.

## Recheck after separating explicit semantic children

The shortcut routing recheck exposed four already-existing failures in
the navigation-focus target. Removing the new shortcut maps did not
change any of them. The passive semantics annotation had become the
outer widget of the navigation scope while the actual focus node moved
to its inner child. The autofocus marker and the app builder's key were
still on the outside.

The native tree now resolves that known annotation to the real retained
scope anchor. It preserves the accessibility annotation, does not
choose a leaf that did not ask for focus, and lets an explicit editor
autofocus win. The builder key still identifies the same navigation
scope through the transparent annotation.

All four pinned originals and both authored reference cases were rerun.
Twenty focused native tests passed across app focus, stock and scoped
shortcut activation, and route autofocus/restoration. This restores
previously paired cases rather than incrementing coverage.
