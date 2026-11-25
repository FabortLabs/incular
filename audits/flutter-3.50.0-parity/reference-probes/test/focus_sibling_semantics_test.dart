import 'dart:ui' show SemanticsFlag;

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'a focused sibling keeps its semantic identity on neighbor rebuild',
    (tester) async {
      final handle = tester.ensureSemantics();

      final left = FocusNode();
      final right = FocusNode();
      addTearDown(left.dispose);
      addTearDown(right.dispose);
      const leftKey = ValueKey('left focus');
      const rightKey = ValueKey('right focus');
      Widget scene(String rightLabel) => Directionality(
        textDirection: TextDirection.ltr,
        child: Stack(
          children: [
            Focus(key: leftKey, focusNode: left, child: const Text('left')),
            Focus(key: rightKey, focusNode: right, child: Text(rightLabel)),
          ],
        ),
      );
      await tester.pumpWidget(scene('right v1'));
      left.requestFocus();
      await tester.pumpAndSettle();

      final leftBefore = tester.getSemantics(find.byKey(leftKey));
      final rightBefore = tester.getSemantics(find.byKey(rightKey));
      expect(leftBefore.label, 'left');
      expect(rightBefore.label, 'right v1');
      expect(
        leftBefore.getSemanticsData().hasFlag(SemanticsFlag.isFocused),
        isTrue,
      );

      await tester.pumpWidget(scene('right v2'));
      final leftAfter = tester.getSemantics(find.byKey(leftKey));
      final rightAfter = tester.getSemantics(find.byKey(rightKey));
      expect(leftAfter.id, leftBefore.id);
      expect(rightAfter.id, rightBefore.id);
      expect(leftAfter.label, 'left');
      expect(rightAfter.label, 'right v2');
      expect(left.hasPrimaryFocus, isTrue);
      expect(
        leftAfter.getSemanticsData().hasFlag(SemanticsFlag.isFocused),
        isTrue,
      );
      handle.dispose();
    },
  );
}
