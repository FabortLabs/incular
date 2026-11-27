import 'package:flutter/painting.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'an ImageInfo scale changes dimensions without replacing the render image',
    (tester) async {
      final original = (await tester.runAsync(
        () => createTestImage(width: 20, height: 10),
      ))!;
      addTearDown(original.dispose);
      RenderImage? retained;

      for (final (double scale, String label, Size expected) in [
        (1.0, 'native pixels', const Size(20, 10)),
        (2.0, '2x image', const Size(10, 5)),
        (2.0, 'another debug label', const Size(10, 5)),
        (1.0, 'restored', const Size(20, 10)),
      ]) {
        final ImageInfo info = ImageInfo(
          image: original.clone(),
          scale: scale,
          debugLabel: label,
        );
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: Align(
              alignment: Alignment.topLeft,
              child: RawImage(image: info.image.clone(), scale: info.scale),
            ),
          ),
        );
        info.dispose();
        final RenderImage current = tester.renderObject(find.byType(RawImage));
        expect(current, same(retained ??= current));
        expect(current.image!.isCloneOf(original), isTrue);
        expect(current.size, expected);
      }
    },
  );
}
