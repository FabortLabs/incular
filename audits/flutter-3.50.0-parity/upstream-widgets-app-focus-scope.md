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
