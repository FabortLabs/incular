import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/foundation.dart';
import 'package:flutter/painting.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

class _FailureBundle extends CachingAssetBundle {
  _FailureBundle({required this.missing});

  final bool missing;

  @override
  Future<ByteData> load(String key) async {
    if (missing) {
      throw FlutterError('bundle did not contain $key');
    }
    return ByteData(8);
  }
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  for (final missing in [true, false]) {
    test('bundle failure leaves pending entry only for decode errors ($missing)', () async {
      final cache = PaintingBinding.instance.imageCache;
      final configuration = ImageConfiguration.empty;
      final provider = ExactAssetImage('asset.png', bundle: _FailureBundle(missing: missing));
      final key = await provider.obtainKey(configuration);
      final stream = provider.resolve(configuration);
      final outcome = Completer<(bool, bool)>();
      final listener = ImageStreamListener(
        (image, _) {
          image.dispose();
          outcome.completeError(StateError('expected an image error'));
        },
        onError: (_, _) {
          final pending = cache.statusForKey(key).pending;
          outcome.complete((pending, cache.evict(key)));
        },
      );
      addTearDown(() {
        stream.removeListener(listener);
        cache.evict(key);
      });
      stream.addListener(listener);

      expect(cache.statusForKey(key).pending, isTrue);
      expect(await outcome.future, (!missing, !missing));
      expect(cache.statusForKey(key).pending, isFalse);
      expect(cache.statusForKey(key).keepAlive, isFalse);
    });
  }

  test('failed asynchronous decode reuses its pending completer until eviction', () async {
    final provider = ExactAssetImage('asset.png', bundle: _FailureBundle(missing: false));
    final configuration = ImageConfiguration.empty;
    final key = await provider.obtainKey(configuration);
    final cache = PaintingBinding.instance.imageCache;
    final first = provider.resolve(configuration);
    final failure = Completer<void>();
    final firstListener = ImageStreamListener(
      (image, _) => image.dispose(),
      onError: (_, _) => failure.complete(),
    );
    first.addListener(firstListener);
    addTearDown(() {
      first.removeListener(firstListener);
      cache.evict(key);
    });

    await failure.future;
    expect(cache.statusForKey(key).pending, isTrue);
    expect(cache.statusForKey(key).keepAlive, isFalse);
    final second = provider.resolve(configuration);
    expect(second.completer, same(first.completer));
    var repeated = false;
    final secondListener = ImageStreamListener(
      (image, _) => image.dispose(),
      onError: (_, _) => repeated = true,
    );
    second.addListener(secondListener);
    expect(repeated, isTrue);
    second.removeListener(secondListener);
    expect(cache.evict(key), isTrue);
    expect(cache.statusForKey(key).pending, isFalse);
  });
}
