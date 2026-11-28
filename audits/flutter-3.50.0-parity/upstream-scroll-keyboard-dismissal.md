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
