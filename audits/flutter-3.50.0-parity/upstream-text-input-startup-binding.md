# Default inbound text-input reply

Pinned `services/binding_test.dart:134` passes. The startup request is
literally `TextInput.requestElementsInRect` on `flutter/textinput`,
with null arguments and no editor attached. Flutter replies with the
encoded JSON method success envelope `[null]`, not a missing reply.
The authored Flutter probe also confirms that the distinct recognized
`TextInputClient.requestElementsInRect` returns an empty list when
there are no handwriting targets.

An opt-in Flutter embedder can install `FlutterTextInputBinding` when
creating its runtime. The native test sends the original bytes through
the installed inbound messenger and checks the exact envelope and the
absence of spurious text-input commands. A real configured handwriting
target returns its geometry before any editor attaches. A control
then attaches an editor and checks that this same receiver rejects
an edit for a different client, applies a valid edit, and returns a
missing method to an unknown request while attached. A retired
runtime does not remain alive because of its channel handler.

The binding shares the existing supported decoded operations. Separate
delta-client, autofill, and ordinary IME action wire dispatch do not
follow from this startup comparison and are not claimed here. Native
keyboard adapters do not need to install the Flutter protocol receiver.

Two mounted native tests and 27 adjacent runtime, Scribble and JSON
method-channel tests passed.
