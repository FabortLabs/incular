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

## Dispatch, IME filtering and repeat state

Four original registrations passed for Windows pressed-modifier
reconciliation (including generic-only Shift/Control/Alt), repeat
detection, and VK_PROCESSKEY suppression. The latter passed in the
pinned typed-then-raw variant. The local
`flutter350_windows_key_message` target uses the mounted legacy
Focus handler and the window raw listener, confirms neither sees
the IME down or its ordinary virtual-key release, and confirms the
claimed handled result. Suppression is tracked by physical scan
identity rather than logical key, and does not ignore unrelated keys
or an actually delivered typed packet. Duplicate raw downs report
repeat until release. Only explicitly raw-only packets can reconstruct
missing modifier transitions; independently supplied typed packets do not.
