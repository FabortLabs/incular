import 'package:flutter/animation.dart' show AlwaysStoppedAnimation;
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('opacity thresholds and partial factors use layer alpha bytes', (
    tester,
  ) async {
    for (final (opacity, byte) in <(double, int)>[
      (0, 0),
      (0.49 / 255, 0),
      (0.51 / 255, 1),
      (0.1, 26),
      (0.5, 128),
      (0.7, 179),
      (0.9, 230),
      (0.99, 252),
      (1, 255),
    ]) {
      await tester.pumpWidget(
        Opacity(
          key: ValueKey(opacity),
          opacity: opacity,
          child: const ColoredBox(
            color: Color(0xFFFFFFFF),
            child: SizedBox(width: 1, height: 1),
          ),
        ),
      );
      final render = tester.renderObject<RenderOpacity>(find.byType(Opacity));
      expect(render.needsCompositing, byte != 0);
      expect(render.paintsChild(render.child!), byte != 0);
      if (byte == 0) {
        expect(render.debugLayer, isNull);
      } else {
        final layer = render.debugLayer! as OpacityLayer;
        expect(layer.alpha, byte);
      }
    }
  });
  testWidgets(
    'tiny animated opacity maintains eligibility but not a raster layer',
    (tester) async {
      const value = AlwaysStoppedAnimation<double>(0.49 / 255);
      await tester.pumpWidget(
        const FadeTransition(
          opacity: value,
          child: ColoredBox(
            color: Color(0xFFFFFFFF),
            child: SizedBox(width: 1, height: 1),
          ),
        ),
      );
      final render = tester.renderObject<RenderAnimatedOpacity>(
        find.byType(FadeTransition),
      );
      expect(render.paintsChild(render.child!), isTrue);
      expect(render.needsCompositing, isFalse);
      expect(render.debugLayer, isNull);
    },
  );
}
