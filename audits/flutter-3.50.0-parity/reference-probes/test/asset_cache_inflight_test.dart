import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

ByteData data(String text) =>
    ByteData.sublistView(Uint8List.fromList(utf8.encode(text)));

class _DeferredBundle extends CachingAssetBundle {
  final first = Completer<ByteData>();
  int reads = 0;

  @override
  Future<ByteData> load(String key) {
    final n = reads++;
    return n == 0 ? first.future : Future.value(data('$key:$n'));
  }
}

void main() {
  for (final all in [false, true]) {
    test('an old string future does not refill after all=$all', () async {
      final root = _DeferredBundle();
      final pending = root.loadString('fixture');
      all ? root.clear() : root.evict('fixture');
      root.first.complete(data('fixture:0'));
      expect(await pending, 'fixture:0');
      expect(await root.loadString('fixture'), 'fixture:1');
      expect(await root.loadString('fixture'), 'fixture:1');
      expect(root.reads, 2);
    });

    test(
      'Flutter currently re-caches a late structured result after all=$all',
      () async {
        final root = _DeferredBundle();
        Future<String> parse(String source) async => source;
        final pending = root.loadStructuredData('fixture', parse);
        all ? root.clear() : root.evict('fixture');
        root.first.complete(data('fixture:0'));
        expect(await pending, 'fixture:0');
        expect(await root.loadStructuredData('fixture', parse), 'fixture:0');
        expect(root.reads, 1);
      },
    );
  }
}
