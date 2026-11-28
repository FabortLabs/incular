# Windows legacy keyboard packets

## Scan codes, virtual keys and modifiers

The pinned `RawKeyEventDataWindows` registrations in
`services/raw_keyboard_test.dart` cover individual and combined
modifier masks, printable/control/navigation/modifier keys, value
equality, and Dart diagnostic formatting. Incular's
`flutter350_windows_raw_keyboard` tests replay the same packet fields
and expected numeric identities. The additional keypad and unknown
code probes cover precedence of keypad identity over a printable
label, physical location, and Windows-plane fallback IDs.

The three tables are reproducible from pinned Flutter source with
`extract-windows-raw-key-catalog.py`. The original Dart diagnostic
format is partial: all four fields are preserved but the Rust Debug
spelling differs. These results are framework packet tests, not a
native Windows desktop or a change to Incular's ordinary host ingress.
