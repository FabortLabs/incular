import 'dart:ui' as ui;

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

class _DelayedImage extends ImageStreamCompleter {
  void finish(ui.Image image) => setImage(ImageInfo(image: image));
}

Future<ui.Image> _image() => createTestImage(width: 8, height: 8, cache: false);

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('Evicting another key does not discard an active image', () async {
    final cache = ImageCache();
    addTearDown(cache.clearLiveImages);
    addTearDown(cache.clear);
    final first = _DelayedImage();
    final second = _DelayedImage();
    expect(cache.putIfAbsent('first', () => first), same(first));
    expect(cache.putIfAbsent('second', () => second), same(second));
    second.finish(await _image());
    expect(cache.evict('second'), isTrue);
    expect(cache.evict('missing'), isFalse);
    expect(cache.statusForKey('first').pending, isTrue);
    first.finish(await _image());
    expect(cache.statusForKey('first').keepAlive, isTrue);
    expect(cache.putIfAbsent('first', () => throw StateError('unexpected reload')), same(first));
  });

  test('Evicted old completion does not replace the same key', () async {
    final cache = ImageCache();
    addTearDown(cache.clearLiveImages);
    addTearDown(cache.clear);
    final old = _DelayedImage();
    final replacement = _DelayedImage();
    final listener = ImageStreamListener((info, _) => info.dispose());
    old.addListener(listener);
    addTearDown(() => old.removeListener(listener));

    expect(cache.putIfAbsent('asset', () => old), same(old));
    expect(cache.evict('asset'), isTrue);
    expect(cache.statusForKey('asset').pending, isFalse);
    expect(cache.putIfAbsent('asset', () => replacement), same(replacement));
    replacement.finish(await _image());
    old.finish(await _image());
    expect(cache.statusForKey('asset').keepAlive, isTrue);
    expect(cache.putIfAbsent('asset', () => throw StateError('unexpected reload')), same(replacement));
  });
}
