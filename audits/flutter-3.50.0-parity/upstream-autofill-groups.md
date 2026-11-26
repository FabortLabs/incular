# Retained autofill group membership and disposal

All four pinned `widgets/autofill_group_test.dart` controls paired with
neutral EditableText widgets. The nearest group lists enabled mounted
clients only, and a nested group does not leak its clients into a parent.
Changing a retained field's hints to and from null changes eligibility.
Removing it eliminates its old ID. A globally keyed field moved across
groups keeps its real retained element/client ID and appears only in the
new group.

The runtime observes the original keyed disposal sequence as
commit, cancel, no message for an inner group, then commit. Each action
becomes the same boolean Flutter method call when using the channel
adapter. Replacing a leaf or unrelated inherited wrapper does not
silently convert it into the same retained group. Android's host seam
maps the finish action to AutofillManager on supported API levels;
this audit did not exercise a device password manager. Winit and the
web hidden IME input have no explicit save/cancel operation.

This batch covers mounted scope and disposal. See
[the mounted channel comparison](upstream-mounted-autofill.md) for
tagged update routing into those actual editors.
