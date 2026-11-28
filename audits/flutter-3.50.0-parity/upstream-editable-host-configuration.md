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

## Host acknowledgements and local edits

- **10672**: An accepted host value is not echoed. A host proposal modified
  by the formatter produces one correction with the downstream collapsed
  selection at UTF-16 offset 20. Each subsequent controller assignment
  produces one update, including when it is set to the previously rejected
  proposal. Once settled, another publication is silent. Incular's test
  samples the mounted host bridge synchronously after each edit.
- **10423, 10584, 10625** are partial. First show follows geometry, style,
  and the initial editing state. An accepted host edit sends no reply,
  while a direct controller edit sends one state with its text and exact
  selection. Flutter's fixture additionally observes an initial selection
  adjustment and second show. Incular's initial state and caret are already
  settled on its first show, so the complete startup trace is not identical.

The matching native checks reuse the actual focused editor, host controls,
wire encoder and inbound editing handler in
`flutter350_full_text_input_channel`. No Android IME driver or physical
mobile device is implied by the comparison.

## Nested batches do not leak intermediate text to the host

Pinned **10174** passes. The native test samples the real wire publisher
after each local edit and after the inner batch closes. Local reads show
`new change 1`, `new change 2`, and `new change 3` in order, the wire stays
silent through all three samples, and closing the outer scope publishes
exactly one `TextInput.setEditingState` for `new change 3`. Geometry and
incoming editing packets are not blocked. A separate native test switches
editors mid-batch: the next attachment remains independent and receives
only its own value.

Pinned **10218** and **10238** check Dart debug assertions for unmatched
begin/end calls. Incular intentionally uses a lexical Rust scope, not a
public begin/end pair; retiring an attachment during the scope is safe
and does not reproduce Dart's disposal assertion. Those two differences
are not counted as paired.
