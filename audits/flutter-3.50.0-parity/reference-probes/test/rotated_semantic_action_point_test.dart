import 'dart:math' as math;

import 'package:flutter/semantics.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('a rotated action excludes the corners of its global bounds', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    try {
      const action = CustomSemanticsAction(label: 'rotate target');
      final id = CustomSemanticsAction.getIdentifier(action);
      var called = 0;
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Center(
            child: Transform.rotate(
              angle: math.pi / 4,
              child: Semantics(
                label: 'rotated',
                customSemanticsActions: {action: () => called++},
                child: const SizedBox(width: 40, height: 40),
              ),
            ),
          ),
        ),
      );
      final center = tester.getCenter(find.byType(Semantics));
      final owner = tester.binding.pipelineOwner.semanticsOwner!;
      void dispatch(Offset point) => owner.performActionAt(
        point * tester.view.devicePixelRatio,
        SemanticsAction.customAction,
        id,
      );
      dispatch(center + const Offset(22, 22));
      expect(called, 0);
      dispatch(center + const Offset(0, -20));
      expect(called, 1);
    } finally {
      semantics.dispose();
    }
  });
}
