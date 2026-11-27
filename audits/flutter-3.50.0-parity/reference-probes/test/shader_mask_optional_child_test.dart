import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'a mask only has a layer and resolves a shader when it has a child',
    (WidgetTester tester) async {
      final calls = <ui.Rect>[];
      ui.Shader shader(ui.Rect bounds) {
        calls.add(bounds);
        return ui.Gradient.linear(bounds.topLeft, bounds.bottomRight, const [
          ui.Color(0xffff0000),
          ui.Color(0xffff0000),
        ]);
      }

      Widget scene(Widget? child) => Center(
        child: ShaderMask(shaderCallback: shader, child: child),
      );

      await tester.pumpWidget(scene(null));
      final render = tester.renderObject<RenderShaderMask>(
        find.byType(ShaderMask),
      );
      expect(render.size, Size.zero);
      expect(render.layer, isNull);
      expect(calls, isEmpty);

      const child = SizedBox.square(
        dimension: 20,
        child: ColoredBox(color: ui.Color(0xffffffff)),
      );
      await tester.pumpWidget(scene(child));
      expect(tester.renderObject(find.byType(ShaderMask)), same(render));
      expect(render.size, const Size(20, 20));
      final firstLayer = render.layer;
      expect(firstLayer, isNotNull);
      expect(calls, [const ui.Rect.fromLTWH(0, 0, 20, 20)]);

      await tester.pumpWidget(scene(null));
      expect(render.size, Size.zero);
      expect(render.layer, isNull);
      expect(calls, hasLength(1));

      await tester.pumpWidget(scene(child));
      expect(render.layer, isNotNull);
      expect(render.layer, isNot(same(firstLayer)));
      expect(calls, hasLength(2));
    },
  );
}
