import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test(
    'FileImage reloads a path after an initially empty file gets written',
    () async {
      final directory = await Directory.systemTemp.createTemp(
        'file-image-retry-',
      );
      addTearDown(() => directory.delete(recursive: true));
      final file = await File('${directory.path}/image.png').create();
      final provider = FileImage(file, scale: 2);
      final cache = PaintingBinding.instance.imageCache;
      addTearDown(() => cache.evict(provider));

      final first = provider.resolve(ImageConfiguration.empty);
      expect(cache.statusForKey(provider).pending, isTrue);
      final error = Completer<Object>();
      final firstListener = ImageStreamListener((info, synchronous) {
        info.dispose();
        error.completeError(StateError('An empty file has no decoded frame'));
      }, onError: (exception, stack) => error.complete(exception));
      first.addListener(firstListener);
      addTearDown(() => first.removeListener(firstListener));
      expect(await error.future, isA<StateError>());
      expect(cache.statusForKey(provider).untracked, isTrue);

      await file.writeAsBytes(base64Decode(png));
      final second = provider.resolve(ImageConfiguration.empty);
      expect(cache.statusForKey(provider).pending, isTrue);
      final frame = Completer<(int, int, double)>();
      final secondListener = ImageStreamListener((info, synchronous) {
        frame.complete((info.image.width, info.image.height, info.scale));
        info.dispose();
      }, onError: frame.completeError);
      second.addListener(secondListener);
      addTearDown(() => second.removeListener(secondListener));
      expect(await frame.future, (8, 8, 2));
    },
  );
}

const png =
    'iVBORw0KGgoAAAANSUhEUgAAAAgAAAAICAYAAADED76LAAAAEklEQVR42mOQiTv4Hx9mGBkKAB3PjoF7MRzrAAAAAElFTkSuQmCC';
