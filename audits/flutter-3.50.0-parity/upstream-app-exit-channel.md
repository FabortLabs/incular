# Application exit protocol

The pinned Flutter 3.50.0 cases `services/binding_test.dart:146,163`
passed. The native adapter sends the required request to
`flutter/platform`, using the original method name, required type,
and default code zero. It decodes the mock engine's cancel result.
For the reverse request, the real inbound JSON method channel encodes
the binding's default exit response in a success envelope.

Controls distinguish a null or missing result (cancel), other
successful objects (exit unless explicitly canceled), a non-default
signed code, a custom cancel decision, and unrelated platform calls.
The adapter is opt-in and does not replace other platform handlers
or exit a process. Fifteen selected native channel and lifecycle
tests passed.
