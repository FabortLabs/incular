import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

class _Registrar extends FontLoader {
  _Registrar() : super('Sliced');

  final first = Completer<void>();
  final delivered = <(String, List<int>)>[];

  @override
  Future<void> loadFont(Uint8List bytes, String family) async {
    delivered.add((family, bytes.toList()));
    if (delivered.length == 1) {
      await first.future;
    }
  }
}

void main() {
  test(
    'font registration receives a subview and completes before the next asset',
    () async {
      final loader = _Registrar();
      final padded = Uint8List.fromList([80, 10, 20, 30, 90]);
      loader.addFont(Future.value(ByteData.view(padded.buffer, 1, 3)));
      loader.addFont(
        Future.value(ByteData.view(Uint8List.fromList([2]).buffer)),
      );
      final load = loader.load();
      await Future<void>.delayed(Duration.zero);

      expect(loader.delivered.length, 1);
      expect(loader.delivered[0].$1, 'Sliced');
      expect(loader.delivered[0].$2, [10, 20, 30]);
      expect(() => loader.addFont(Future.value(ByteData(0))), throwsStateError);

      loader.first.complete();
      await load;
      expect(loader.delivered.length, 2);
      expect(loader.delivered[1].$2, [2]);
      await expectLater(loader.load(), throwsStateError);
    },
  );
}
