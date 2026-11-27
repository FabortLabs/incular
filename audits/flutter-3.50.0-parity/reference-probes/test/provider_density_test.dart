import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

Future<ImageInfo> first(ImageProvider<Object> provider) {
  final completer = Completer<ImageInfo>();
  final stream = provider.resolve(ImageConfiguration.empty);
  late final ImageStreamListener listener;
  listener = ImageStreamListener(
    (image, _) {
      stream.removeListener(listener);
      completer.complete(image);
    },
    onError: (error, trace) {
      stream.removeListener(listener);
      completer.completeError(error, trace);
    },
  );
  stream.addListener(listener);
  return completer.future;
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('memory and file density changes logical size without changing decoded size', () async {
    final bytes = base64Decode(
      'iVBORw0KGgoAAAANSUhEUgAAAAgAAAAICAYAAADED76LAAAAEklEQVR42mOQiTv4Hx9mGBkKAB3PjoF7MRzrAAAAAElFTkSuQmCC',
    );
    final folder = await Directory.systemTemp.createTemp('incular-density-');
    try {
      final file = await File.fromUri(folder.uri.resolve('image.png'))
          .writeAsBytes(bytes);
      for (final (provider, scale) in <(ImageProvider<Object>, double)>[
        (MemoryImage(bytes), 1),
        (MemoryImage(bytes, scale: 2), 2),
        (FileImage(file), 1),
        (FileImage(file, scale: 4), 4),
        (ResizeImage(MemoryImage(bytes, scale: 2), width: 4, height: 4), 2),
      ]) {
        final frame = await first(provider);
        try {
          expect(frame.scale, scale);
          final size = provider is ResizeImage ? 4 : 8;
          expect((frame.image.width, frame.image.height), (size, size));
        } finally {
          frame.dispose();
        }
      }
    } finally {
      PaintingBinding.instance.imageCache.clear();
      PaintingBinding.instance.imageCache.clearLiveImages();
      await folder.delete(recursive: true);
    }
  });
}
