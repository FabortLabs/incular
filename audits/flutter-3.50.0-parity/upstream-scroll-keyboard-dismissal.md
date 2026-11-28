# Keyboard dismissal driven by scrolling

Pinned `widgets/single_child_scroll_view_test.dart:1038` passed.
The corresponding native touch drag starts on a focused EditableText
and passes the original manual/onDrag contrast. The scroll view now
uses its explicit setting, else the nearest ScrollConfiguration;
the default is manual. Rebuilding a retained viewport can switch
policies without replacing the focus or scroll controller.

Dismissal goes through an actual scroll update with drag details,
not pointer-down, overscroll, programmatic motion or a wheel event.
A scope without a focused descendant cannot dismiss an editor in a
different scope. A native blur retires the text-input client and sends
Hide. The authored Flutter scope/notification probe and all eleven
focused native checks passed. These checks cover SingleChildScrollView,
not every sliver-based scroll descriptor.

## List, grid, and custom scroll views

The next fifteen originals in pinned `widgets/scroll_view_test.dart`
passed: the explicit onDrag variants and the default manual variants
for ListView, ListView.builder, ListView.separated, GridView,
GridView.builder, GridView.count and GridView.extent, plus the
CustomScrollView onDrag case with DragStartBehavior.down.

Native tests mount fifty actual editors, focus one, and drag from
that editor by the reference's 40 logical pixels. They verify an
actual offset change and whether the same editor retains focus.
Extra cases cover inherited onDrag and an explicit manual override.
A raw Viewport remains a renderer, rather than installing a scroll
view's dismissal listener. The named ListView.custom and
GridView.custom child-delegate constructors are not credited here.
All 14 focused native keyboard and zero-area tests passed.

### Custom child delegates

Pinned :258, :499, :616, and :851 also passed. Incular now exposes
the corresponding ListView.custom and GridView.custom constructors.
Each uses the child delegate's own repaint, semantics and retention
wrappers rather than adding a second set of wrappers. Mounted
manual and onDrag cases match with the default child builder delegate.
A 10,000-child finite delegate is not built eagerly: before layout
there are no builder calls, and after layout fewer than 40 children
have been built in either case. This does not claim an open-ended
delegate with no known count. All twelve focused native tests passed.
