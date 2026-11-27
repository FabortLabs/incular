import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('a translated shader mask preserves the ancestor semantic clip', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    await tester.pumpWidget(
      Align(
        alignment: Alignment.topLeft,
        child: ClipRect(
          child: Transform.translate(
            offset: const ui.Offset(10, 5),
            child: ShaderMask(
              shaderCallback: (ui.Rect bounds) => ui.Gradient.linear(
                bounds.topLeft,
                bounds.bottomRight,
                const <ui.Color>[ui.Color(0xFFFFFFFF), ui.Color(0xFFFFFFFF)],
              ),
              child: Semantics(
                label: 'masked child',
                textDirection: ui.TextDirection.ltr,
                container: true,
                button: true,
                child: const SizedBox(width: 40, height: 40),
              ),
            ),
          ),
        ),
      ),
    );
    final RenderShaderMask mask = tester.renderObject(find.byType(ShaderMask));
    expect(mask.size, const ui.Size(40, 40));
    final SemanticsNode node = tester.getSemantics(
      find.bySemanticsLabel('masked child'),
    );
    expect(node.rect, const ui.Rect.fromLTWH(0, 0, 30, 35));
    semantics.dispose();
  });
}
