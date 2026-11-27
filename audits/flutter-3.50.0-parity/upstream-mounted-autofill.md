# Tagged autofill updates in mounted editors

The authored Flutter VM control uses real EditableText widgets and the
Flutter text-input binary messenger. A sender-zero update fills the
nonfocused field in the active group, ignores the nested group, leaves
read-only text unchanged, and does not move focus. It emits one change
callback per accepted replacement. A disabled field stops accepting
its existing tag.

Incular's optional Flutter channel adapter now obtains grouped
configuration from the actual mounted runtime. It includes other
eligible fields without focusing them and routes edits to their real
editing controllers. The UTF-16 offsets around an emoji become valid
Rust byte boundaries. Reverse selection, visual affinity, and the
directional flag survive; a hidden nonfocused editor does not retain
an IME composing range, as observed on the pinned reference. An unset
selection remains unset, rather than becoming a position-zero caret.

Extra Incular controls reject a surrogate-splitting batch before
changing any controller and isolate tags by runtime as well as retained
element generation. Nothing here verifies an actual browser or device
password-manager session. This is integration evidence for the
previously paired services case, not an extra upstream registration.

## Autofill leaves focus on the original editor

The pinned `widgets/editable_text_test.dart:10531` registration passed.
The native test taps the first of two mounted neutral editors, then
sends a tagged editing value to the disconnected second editor through
the production autofill receiver. Its password value and collapsed
selection are updated without changing the active editor or focus
nodes. The first editor remains untouched. This verifies the retained
contract, not a native OS autofill UI.
