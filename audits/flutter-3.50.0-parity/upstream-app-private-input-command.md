# Application-private input-method messages

Pinned source: Flutter 3.50.0, `ab2912035e160d9cad6306df307576da6128ebdb`,
`services/text_input.dart` and the forwarding hook in
`widgets/editable_text.dart`. These are Flutter JSON messages for an active
client, not operating-system or process commands.

The six exact `services/text_input_test.dart` cases from lines 902, 927, 952,
980, 1005, and 1030 passed on the pinned SDK. The native
`flutter350_app_private_command` test sends the original action and field
values through the optional channel to a real mounted editor. It verifies
verbatim strings, floats, homogeneous arrays and an empty map for omitted
data. The app callback does not change editing state or focus by default.

The additional pinned `editor_private_input_command_test.dart` drives two
mounted editors. It verifies old-client rejection after a focus transfer,
explicit null normalization, mixed JSON values and no extra host commands.
Native tests also cover malformed active messages, callback replacement on the
same client and panic reporting that leaves the connection alive. The
channel tests do not claim that the native Android launcher forwards an
Android-specific callback into this Flutter JSON adapter.

Pinned `widgets/editable_text_test.dart:930` also passed. An
actual mounted editor with no private-command callback receives the
original message after its controller is updated to `test`. The
optional message succeeds without changing the editing value, focus
or pending keyboard traffic. The focused native suite covers both
the original debug-current sender and the actual attached client;
all five checks passed.
