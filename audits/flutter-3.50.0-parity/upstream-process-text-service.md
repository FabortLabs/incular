# Platform process-text commands

Pinned `services/process_text_test.dart:11, :29, :55` pass.
The native default service sends a StandardMethodCodec request
to flutter/processtext, with null for querying or the exact
action, selected text and read-only arguments for processing.
Its mock host decodes the method call and returns a real encoded
envelope. The service yields the two ordered action IDs and
labels, then the transformed text for the first and null for
the second.

Native controls distinguish optional/invalid query replies
(empty action list) from a malformed result for a specific action
(an error). Sixteen focused native checks passed. Android app
discovery and invoking a physical device's share/process targets
are owned by the host and were not claimed here.
