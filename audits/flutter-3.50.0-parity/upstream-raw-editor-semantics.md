# Raw editable semantics

Pinned `widgets/editable_text_test.dart` :4729, :4779 and :4844 passed.
The mounted native editor starts as a focusable, unfocused text field.
Tapping it sets focused on the same semantic node. Updating maxLines from
1 to 3 keeps that node and reports multiline. The native TextArea role
maps to Flutter's isTextField flag as well as its separate isMultiline
flag. Independently, changing the controller's plain text while the field
is unfocused replaces the published value without changing identity or
focus. All eight focused native tests passed, including neighboring
obscuring and cursor-action controls.
