# Read-only editor focus and input connections

Reference: Flutter 3.50.0 (`ab2912035e160d9cad6306df307576da6128ebdb`),
`packages/flutter/test/widgets/editable_text_test.dart`. Incular equivalent:
`cargo test -p incular-runtime --test flutter350_editable_read_only_connection`.
These cases observe the mounted focus node, the controller, and the real
runtime's outgoing host commands.

## Read-only fields remain in native tab order

- **3679**: Tab moves from the autofocus editor to the read-only editor and
  the next Tab wraps to the first editor. The upstream reference passed all
  six target platform variants. Incular tests each corresponding native
  shortcut profile; Fuchsia intentionally shares the Android table.

## Native input connection closes independently of focus

- **3557**: A real pointer click focuses a read-only editor. Selecting the
  first five characters still works without an IME client or show request.
- **3596 / 3639**: A native editing-state event cannot change read-only text,
  alter the current selection, open composition, or invoke its formatter.
- **3735**: Removing read-only from the focused editor opens a client with the
  current text, selection, and writable configuration. Restoring the flag
  clears that client without moving focus or losing selection. Previously,
  the runtime issued a native attach/show command even for read-only fields.

The last four upstream registrations branch on `kIsWeb`. Their native
behavior was run and compared; the Flutter browser-specific connection,
selection-only editing and configuration paths were not verified and are
not counted as paired here. This fix does not change the WASM branch.
