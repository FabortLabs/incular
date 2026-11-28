# Live keyboard configuration

Reference: Flutter 3.50.0 (`ab2912035e160d9cad6306df307576da6128ebdb`),
`packages/flutter/test/widgets/editable_text_test.dart`. The Incular
counterpart is `flutter350_editable_host_configuration`.

## Obscuring and view changes update an existing client

- **3777**: The initial mounted client's `obscureText` is true. Updating the
  same focused widget makes it false on the host without a detach, a new
  `setClient`, or a keyboard show.
- **3859**: The first attachment carries view ID 14. Changing it to 15
  emits precisely one `TextInput.updateConfig`. The client ID, text selection
  and focus stay the same.

The optional Flutter protocol host attaches with the runtime's configuration
and publishes subsequent changes through
`Runtime::publish_flutter_text_input_configuration`. This is distinct from
the native IME command stream. Host-owned options (the test uses its delta
model opt-in) remain supplied by the host after configuration changes.
Unchanged configurations are silent. The explicit lower-level `update` API
still supports deliberately resending a configuration.
