import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

ui.Image _image(int width, int height) {
  final recorder = ui.PictureRecorder();
  ui.Canvas(recorder).drawColor(const ui.Color(0xFFFFFFFF), ui.BlendMode.src);
  final picture = recorder.endRecording();
  try {
    return picture.toImageSync(width, height);
  } finally {
    picture.dispose();
  }
}

void main() {
  testWidgets(
    'RenderImage only remeasures a replacement that can change size',
    (tester) async {
      final a = _image(10, 10);
      final sameSize = _image(10, 10);
      final differentSize = _image(20, 10);
      addTearDown(a.dispose);
      addTearDown(sameSize.dispose);
      addTearDown(differentSize.dispose);

      await tester.pumpWidget(Center(child: RawImage(image: a)));
      final render = tester.renderObject<RenderImage>(find.byType(RawImage));
      expect(render.size, const Size(10, 10));
      render.image = sameSize.clone();
      expect(render.debugNeedsLayout, isFalse);
      expect(render.debugNeedsPaint, isTrue);
      await tester.pump();
      render.image = differentSize.clone();
      expect(render.debugNeedsLayout, isTrue);
      await tester.pump();
      expect(render.size, const Size(20, 10));

      await tester.pumpWidget(
        Center(child: RawImage(image: a, width: 24, height: 18)),
      );
      expect(
        tester.renderObject<RenderImage>(find.byType(RawImage)),
        same(render),
      );
      render.image = differentSize.clone();
      expect(render.debugNeedsLayout, isFalse);
      expect(render.debugNeedsPaint, isTrue);
      await tester.pump();
      render.image = null;
      expect(render.debugNeedsLayout, isFalse);
      await tester.pump();
      expect(render.size, const Size(24, 18));
    },
  );

  testWidgets(
    'An empty RawImage hits its half-open box but has no zero-area hit',
    (tester) async {
      await tester.pumpWidget(
        const Center(child: RawImage(width: 20, height: 10)),
      );
      final render = tester.renderObject<RenderImage>(find.byType(RawImage));
      bool hits(double x, double y) =>
          render.hitTest(BoxHitTestResult(), position: Offset(x, y));
      expect(hits(0, 0), isTrue);
      expect(hits(19.999, 9.999), isTrue);
      expect(hits(20, 5), isFalse);
      expect(hits(5, 10), isFalse);
      expect(hits(-0.001, 5), isFalse);
      await tester.pumpWidget(
        const Center(child: RawImage(width: 0, height: 10)),
      );
      expect(hits(0, 5), isFalse);
    },
  );
}
