import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

class _Buffers extends ui.ChannelBuffers {
  @override
  void sendChannelUpdate(String name, {required bool listening}) {}
}

ByteData call(String method, List<Object> arguments) =>
    const StandardMethodCodec().encodeMethodCall(MethodCall(method, arguments));

void main() {
  test(
    'wire resize keeps the newest messages and legacy resize still works',
    () async {
      final buffer = _Buffers();
      final dropped = <int>[];
      final received = <int>[];
      buffer.handleMessage(call('resize', ['custom', 3, 'future payload']));
      for (var id = 1; id <= 3; ++id) {
        buffer.push('custom', ByteData(1)..setUint8(0, id), (reply) {
          if (reply == null) {
            dropped.add(id);
          }
        });
      }
      final legacy = Uint8List.fromList(utf8.encode('resize\rcustom\r1'));
      buffer.handleMessage(ByteData.sublistView(legacy));
      expect(dropped, [1, 2]);
      buffer.setListener('custom', (data, reply) {
        received.add(data!.getUint8(0));
        reply(ByteData(0));
      });
      expect(received, isEmpty);
      await Future<void>.delayed(Duration.zero);
      expect(received, [3]);
    },
  );

  test(
    'wire overflow toggling preserves null replies to discarded messages',
    () {
      final buffer = _Buffers();
      final dropped = <int>[];
      buffer.handleMessage(call('overflow', ['custom', true]));
      buffer.push('custom', null, (_) => dropped.add(1));
      buffer.push('custom', null, (_) => dropped.add(2));
      expect(dropped, [1]);
      buffer.handleMessage(call('overflow', ['custom', false]));
      buffer.push('custom', null, (_) => dropped.add(3));
      expect(dropped, [1, 2]);
      expect(
        () => buffer.handleMessage(call('resize', ['custom', true])),
        throwsException,
      );
    },
  );
}
