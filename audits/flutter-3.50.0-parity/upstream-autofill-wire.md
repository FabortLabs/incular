# Autofill service configuration and tagged route

All four pinned `services/autofill_test.dart` registrations pair:
enabled empty hints, configuration equality/hash, ordered enabled fields
in `TextInput.setClient`, and a tagged editing update for a nonfocused
field with sender zero. The codec retains the embedder's other input
options and does not map wire offsets to Rust bytes prematurely.

Additional native checks distinguish omitted fields from enabled
empty hints, preserve signed UTF-16 selections and composition, and
ignore unknown, removed and disabled tags. Incular rejects a malformed
batch before applying any values. The value's diagnostic formatter
redacts editing text. Save/commit and cancel have the pinned boolean
wire format.

This is the optional Flutter-compatible channel contract. The retained
widget tree and group-dispose events are covered in
[the widget scope batch](upstream-autofill-groups.md); routing the
channel into mounted editors is separate.

```sh
cargo test --locked -j 1 -p incular-platform --test flutter350_autofill --test flutter350_json_method_codec -- --test-threads=1
```
