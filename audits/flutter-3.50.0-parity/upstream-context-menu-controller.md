# Context menu ownership and retained menu lifecycle

All six pinned `widgets/context_menu_controller_test.dart` registrations
are paired. The neutral `ContextMenuController` supplies its overlay through
a mounted `host` in Rust. It shares one active owner with the built-in
editor toolbar. A second owner replaces the first, and removing a hidden
controller cannot accidentally dismiss the active menu.

Builders run on the next frame, not while calling `show` or
`mark_needs_build`. Showing again while open replaces the content of the
same entry and preserves mounted child state. Removing then showing uses
fresh state, including when both calls occur before one frame.

The native runtime test uses an actual focused editor and inbound toolbar
message. Direct and built-in menus replace each other without changing the
editor's focus, selection, or platform input attachment. Six named widget
tests and eleven targeted toolbar, focus, and spelling regressions passed.
Raster/theme capture and browser-provided menus are not asserted here.
