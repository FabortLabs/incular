import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class HeldImage extends ImageProvider<HeldImage> {
  final completion = Completer<ImageInfo>();
  int resolutions = 0;

  @override
  Future<HeldImage> obtainKey(ImageConfiguration configuration) =>
      SynchronousFuture(this);

  @override
  ImageStreamCompleter loadImage(HeldImage key, ImageDecoderCallback decode) {
    resolutions++;
    return OneFrameImageStreamCompleter(completion.future);
  }
}

void main() {
  test(
    'image and containing decorations omit the error callback from equality',
    () {
      final provider = HeldImage();
      void first(Object _, StackTrace? __) {}
      void second(Object _, StackTrace? __) {}
      final a = DecorationImage(image: provider, onError: first);
      final b = DecorationImage(image: provider, onError: second);
      final c = DecorationImage(image: provider, onError: second, opacity: 0.5);
      expect(a, b);
      expect(a.hashCode, b.hashCode);
      expect(a, isNot(c));

      final boxA = BoxDecoration(image: a);
      final boxB = BoxDecoration(image: b);
      expect(boxA, boxB);
      expect(boxA.hashCode, boxB.hashCode);
      expect(boxA, isNot(BoxDecoration(image: c)));

      final shapeA = ShapeDecoration(shape: const Border(), image: a);
      final shapeB = ShapeDecoration(shape: const Border(), image: b);
      expect(shapeA, shapeB);
      expect(shapeA.hashCode, shapeB.hashCode);
      expect(shapeA, isNot(ShapeDecoration(shape: const Border(), image: c)));
    },
  );

  for (final shape in [false, true]) {
    for (final changePaint in [false, true]) {
      testWidgets(
        'held failure after a rebuild: shape=$shape paint=$changePaint',
        (tester) async {
          final provider = HeldImage();
          final seen = <(String, Object)>[];
          final error = StateError('held image failed');

          Widget scene(String listener, double opacity) {
            final image = DecorationImage(
              image: provider,
              opacity: opacity,
              onError: (Object error, StackTrace? _) =>
                  seen.add((listener, error)),
            );
            return Directionality(
              textDirection: TextDirection.ltr,
              child: Center(
                child: DecoratedBox(
                  decoration: shape
                      ? ShapeDecoration(shape: const Border(), image: image)
                      : BoxDecoration(image: image),
                  child: const SizedBox.square(dimension: 40),
                ),
              ),
            );
          }

          await tester.pumpWidget(scene('old', 1));
          await tester.pumpWidget(scene('new', changePaint ? 0.5 : 1));
          expect(provider.resolutions, 1);
          expect(seen, isEmpty);
          provider.completion.completeError(error);
          await tester.pump();

          // Equal decoration values do not replace Flutter's painter.
          expect(seen, [(changePaint ? 'new' : 'old', error)]);
          expect(tester.takeException(), isNull);
        },
      );
    }
  }
}
