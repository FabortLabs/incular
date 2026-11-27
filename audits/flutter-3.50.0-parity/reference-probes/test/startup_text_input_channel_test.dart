import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('the unbound text-input channel still replies with an envelope', (
    tester,
  ) async {
    final messenger = tester.binding.defaultBinaryMessenger;
    const codec = JSONMethodCodec();

    Future<ByteData> send(String method, Object? arguments) async {
      ByteData? reply;
      await messenger.handlePlatformMessage(
        'flutter/textinput',
        codec.encodeMethodCall(MethodCall(method, arguments)),
        (bytes) => reply = bytes,
      );
      expect(reply, isNotNull);
      return reply!;
    }

    final startup = await send('TextInput.requestElementsInRect', null);
    expect(
      utf8.decode(
        startup.buffer.asUint8List(
          startup.offsetInBytes,
          startup.lengthInBytes,
        ),
      ),
      '[null]',
    );
    expect(codec.decodeEnvelope(startup), isNull);
    final handwriting = await send('TextInputClient.requestElementsInRect', [
      0,
      0,
      40,
      40,
    ]);
    expect(codec.decodeEnvelope(handwriting), isEmpty);
  });
}
