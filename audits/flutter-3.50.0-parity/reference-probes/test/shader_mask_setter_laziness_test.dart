import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'replacing a mounted shader callback defers its work until paint',
    (WidgetTester tester) async {
      final List<int> calls = <int>[0, 0];
      ui.Shader shader(int index, ui.Color color, ui.Rect bounds) {
        calls[index]++;
        expect(bounds, const ui.Rect.fromLTWH(0, 0, 15, 20));
        return ui.Gradient.linear(
          bounds.topLeft,
          bounds.bottomRight,
          <ui.Color>[color, color],
        );
      }

      await tester.pumpWidget(
        Center(
          child: ShaderMask(
            shaderCallback: (ui.Rect bounds) =>
                shader(0, const ui.Color(0xFFFF0000), bounds),
            child: const SizedBox(
              width: 15,
              height: 20,
              child: ColoredBox(color: ui.Color(0xFFFFFFFF)),
            ),
          ),
        ),
      );
      expect(calls, <int>[1, 0]);
      final RenderShaderMask render = tester.renderObject(
        find.byType(ShaderMask),
      );
      render.shaderCallback = (ui.Rect bounds) =>
          shader(1, const ui.Color(0xFF0000FF), bounds);
      render.blendMode = ui.BlendMode.colorBurn;
      expect(calls, <int>[1, 0]);
      await tester.pump();
      expect(calls, <int>[1, 1]);
      expect(tester.renderObject(find.byType(ShaderMask)), same(render));
    },
  );
}
