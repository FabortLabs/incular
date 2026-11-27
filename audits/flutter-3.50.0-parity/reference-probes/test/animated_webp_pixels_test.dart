import 'dart:convert';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/painting.dart' show PaintingBinding;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

const finite =
    'UklGRoQAAABXRUJQVlA4WAoAAAACAAAAAwAAAwAAQU5JTQYAAAAAAAAAAgBBTk1G'
    'KAAAAAAAAAAAAAMAAAMAAMgAAAJWUDhMDwAAAC8DwAAABxD9j/4HIqL/AQBBTk1G'
    'KAAAAAAAAAAAAAMAAAMAAJABAABWUDhMDwAAAC8DwAAABxDR//4HIqL/AQA=';
const partial =
    'UklGRoQAAABXRUJQVlA4WAoAAAASAAAAAwAAAQAAQU5JTQYAAAAAAAAAAABBTk1G'
    'KAAAAAAAAAAAAAEAAAEAAMgAAAJWUDhMDwAAAC8BQAAABxD9j/4HIqL/AQBBTk1G'
    'KAAAAAEAAAAAAAEAAAEAAJABAAJWUDhMDwAAAC8BQAAABxDR//4HIqL/AQA=';

const red = [255, 0, 0, 255];
const blue = [0, 0, 255, 255];
const clear = [0, 0, 0, 0];

List<int> pixel(ByteData data, int width, int x, int y) {
  final offset = (y * width + x) * 4;
  return List.generate(4, (channel) => data.getUint8(offset + channel));
}

void main() {
  test(
    'real WebP plays two red and blue frames with 200 and 400 ms delays',
    () async {
      final codec = await ui.instantiateImageCodec(base64Decode(finite));
      addTearDown(codec.dispose);
      expect(codec.frameCount, 2);
      expect(codec.repetitionCount, 1);
      for (final (milliseconds, color) in [
        (200, red),
        (400, blue),
        (200, red),
        (400, blue),
      ]) {
        final frame = await codec.getNextFrame();
        expect(frame.duration.inMilliseconds, milliseconds);
        final source = frame.image;
        try {
          expect((source.width, source.height), (4, 4));
          final data = (await source.toByteData(
            format: ui.ImageByteFormat.rawStraightRgba,
          ))!;
          for (var y = 0; y < source.height; y++) {
            for (var x = 0; x < source.width; x++) {
              expect(pixel(data, source.width, x, y), color);
            }
          }
        } finally {
          source.dispose();
        }
      }
    },
  );

  test(
    'wrapping an animated WebP starts from a clear compositing canvas',
    () async {
      final codec = await ui.instantiateImageCodec(base64Decode(partial));
      addTearDown(codec.dispose);
      expect(codec.frameCount, 2);
      expect(codec.repetitionCount, -1);
      for (final right in [clear, blue, clear, blue]) {
        final frame = await codec.getNextFrame();
        final source = frame.image;
        try {
          expect((source.width, source.height), (4, 2));
          final data = (await source.toByteData(
            format: ui.ImageByteFormat.rawStraightRgba,
          ))!;
          for (var y = 0; y < source.height; y++) {
            for (var x = 0; x < 2; x++) {
              expect(pixel(data, source.width, x, y), red);
              expect(pixel(data, source.width, x + 2, y), right);
            }
          }
        } finally {
          source.dispose();
        }
      }
    },
  );
  test('disposal clears the previous image rectangle only', () async {
    final bytes = base64Decode(partial);
    final first = [65, 78, 77, 70];
    final index = Iterable<int>.generate(bytes.length - 4).firstWhere(
      (i) => List.generate(
        4,
        (j) => bytes[i + j] == first[j],
      ).every((same) => same),
    );
    bytes[index + 8 + 15] |= 1;
    final codec = await ui.instantiateImageCodec(bytes);
    addTearDown(codec.dispose);
    for (final (left, right) in [(red, clear), (clear, blue)]) {
      final source = (await codec.getNextFrame()).image;
      try {
        final data = (await source.toByteData(
          format: ui.ImageByteFormat.rawStraightRgba,
        ))!;
        expect(pixel(data, source.width, 0, 0), left);
        expect(pixel(data, source.width, 3, 1), right);
      } finally {
        source.dispose();
      }
    }
  });
  testWidgets(
    'Image.memory retains animated WebP canvas size despite cacheWidth',
    (tester) async {
      final frameNumbers = <int?>[];
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Image.memory(
            base64Decode(finite),
            cacheWidth: 2,
            frameBuilder: (context, child, frame, synchronous) {
              frameNumbers.add(frame);
              return child;
            },
          ),
        ),
      );
      expect(frameNumbers.last, isNull);

      Future<List<int>> displayed() async {
        final image = tester.widget<RawImage>(find.byType(RawImage)).image!;
        expect((image.width, image.height), (4, 4));
        return (await tester.runAsync(() async {
          final data = (await image.toByteData(
            format: ui.ImageByteFormat.rawStraightRgba,
          ))!;
          return pixel(data, image.width, 0, 0);
        }))!;
      }

      for (
        var attempt = 0;
        frameNumbers.last == null && attempt < 30;
        attempt++
      ) {
        await tester.runAsync(
          () => Future<void>.delayed(const Duration(milliseconds: 5)),
        );
        await tester.pump();
      }
      expect(frameNumbers.last, 0);
      expect(await displayed(), red);

      // Allow the decoder's I/O thread to finish the prefetched second frame.
      // Pumping here does not advance the framework's fake app clock.
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 60)),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 199));
      expect(frameNumbers.last, 0);
      await tester.pump(const Duration(milliseconds: 1));
      expect(frameNumbers.last, 1);
      expect(await displayed(), blue);
      await tester.pumpWidget(const SizedBox());
      PaintingBinding.instance.imageCache.clear();
      PaintingBinding.instance.imageCache.clearLiveImages();
    },
  );
  testWidgets('live animated memory widgets share only matching cache keys', (
    tester,
  ) async {
    final bytes = base64Decode(finite);
    final copy = base64Decode(finite);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Row(
          children: [
            Image.memory(bytes),
            Image.memory(bytes),
            Image.memory(bytes, cacheWidth: 2),
            Image.memory(copy),
          ],
        ),
      ),
    );
    List<RawImage> mounted() =>
        tester.widgetList<RawImage>(find.byType(RawImage)).toList();

    for (
      var attempt = 0;
      mounted().any((raw) => raw.image == null) && attempt < 40;
      attempt++
    ) {
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 5)),
      );
      await tester.pump();
    }
    final images = mounted().map((raw) => raw.image!).toList();
    expect(images.length, 4);
    expect(images[0].isCloneOf(images[1]), isTrue);
    expect(images[0].isCloneOf(images[2]), isFalse);
    expect(images[0].isCloneOf(images[3]), isFalse);

    await tester.pumpWidget(const SizedBox());
    PaintingBinding.instance.imageCache.clear();
    PaintingBinding.instance.imageCache.clearLiveImages();
  });
}
