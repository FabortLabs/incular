import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test(
    'ResizeImage evicts the failed resize key after its error listener',
    () async {
      final directory = await Directory.systemTemp.createTemp(
        'resize-file-retry-',
      );
      addTearDown(() => directory.delete(recursive: true));
      final file = File('${directory.path}/image.png');
      await file.writeAsString('nonempty but not a raster image');
      final raw = FileImage(file);
      final resized = ResizeImage(raw, width: 4, height: 4);
      final cache = PaintingBinding.instance.imageCache;
      const configuration = ImageConfiguration.empty;
      final resizeKey = await resized.obtainKey(configuration);
      addTearDown(() => cache.evict(raw));
      addTearDown(() => cache.evict(resizeKey));

      Future<Object> observeFailure(
        ImageProvider provider,
        void Function() inListener,
      ) {
        final stream = provider.resolve(configuration);
        final failure = Completer<Object>();
        final listener = ImageStreamListener(
          (frame, sync) {
            frame.dispose();
            failure.completeError(StateError('Invalid input produced a frame'));
          },
          onError: (error, stack) {
            inListener();
            failure.complete(error);
          },
        );
        stream.addListener(listener);
        addTearDown(() => stream.removeListener(listener));
        return failure.future;
      }

      expect(
        (await observeFailure(raw, () {})).toString(),
        contains('Invalid image data'),
      );
      expect(cache.statusForKey(raw).pending, isTrue);

      var pendingDuringCallback = false;
      final failure = await observeFailure(resized, () {
        pendingDuringCallback = cache.statusForKey(resizeKey).pending;
      });
      expect(failure.toString(), contains('Invalid image data'));
      expect(pendingDuringCallback, isTrue);
      await Future<void>.delayed(Duration.zero);
      expect(cache.statusForKey(resizeKey).untracked, isTrue);
      expect(cache.statusForKey(raw).pending, isTrue);

      await file.writeAsBytes(base64Decode(png));
      final recovered = resized.resolve(configuration);
      final result = Completer<(int, int)>();
      final listener = ImageStreamListener((frame, sync) {
        result.complete((frame.image.width, frame.image.height));
        frame.dispose();
      }, onError: result.completeError);
      recovered.addListener(listener);
      addTearDown(() => recovered.removeListener(listener));
      expect(await result.future, (4, 4));
      expect(cache.statusForKey(raw).pending, isTrue);
    },
  );
}

const png =
    'iVBORw0KGgoAAAANSUhEUgAAAAgAAAAICAYAAADED76LAAAAEklEQVR42mOQiTv4Hx9mGBkKAB3PjoF7MRzrAAAAAElFTkSuQmCC';
