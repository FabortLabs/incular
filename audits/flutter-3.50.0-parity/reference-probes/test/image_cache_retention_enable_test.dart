import 'dart:ui' as ui;

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

class _DelayedImage extends ImageStreamCompleter {
  void finish(ui.Image image) => setImage(ImageInfo(image: image));
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('An image that starts with retention off is not kept after limits rise', () async {
    for (final limitByBytes in [false, true]) {
      final cache = ImageCache();
      if (limitByBytes) {
        cache.maximumSizeBytes = 0;
      } else {
        cache.maximumSize = 0;
      }
      final loader = _DelayedImage();
      final stream = cache.putIfAbsent('asset', () => loader)!;
      expect(cache.pendingImageCount, 0);
      cache.maximumSize = 4;
      cache.maximumSizeBytes = 4096;

      var delivered = 0;
      final observer = ImageStreamListener((info, _) {
        expect(info.image.width, 8);
        delivered++;
        info.dispose();
      });
      stream.addListener(observer);
      loader.finish(await createTestImage(width: 8, height: 8, cache: false));
      expect(delivered, 1);
      expect(cache.pendingImageCount, 0);
      expect(cache.currentSize, 0);
      expect(cache.statusForKey('asset').keepAlive, isFalse);
      stream.removeListener(observer);
      cache.clear();
      cache.clearLiveImages();
    }
  });
}
