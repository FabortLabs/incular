import 'dart:ui' show SemanticsAction, SemanticsActionEvent;

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'semantic actions outside a TapRegion also run the target action',
    (tester) async {
      final handle = tester.ensureSemantics();
      try {
        var outside = 0;
        var target = 0;
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: TapRegionSurface(
              child: Row(
                children: [
                  Semantics(
                    label: 'Outside',
                    onTap: () => target++,
                    onLongPress: () => target++,
                    child: const SizedBox(
                      width: 100,
                      height: 50,
                      child: ColoredBox(color: Color(0xff0000ff)),
                    ),
                  ),
                  TapRegion(
                    onTapOutside: (_) => outside++,
                    child: const SizedBox(
                      width: 100,
                      height: 50,
                      child: Text('Inside'),
                    ),
                  ),
                ],
              ),
            ),
          ),
        );
        final node = tester.semantics.find(find.bySemanticsLabel('Outside'));
        for (final action in [SemanticsAction.tap, SemanticsAction.longPress]) {
          outside = 0;
          target = 0;
          tester.binding.platformDispatcher.onSemanticsActionEvent!(
            SemanticsActionEvent(
              type: action,
              nodeId: node.id,
              viewId: tester.view.viewId,
            ),
          );
          await tester.pump();
          expect((outside, target), (1, 1), reason: '$action');
        }
      } finally {
        handle.dispose();
      }
    },
  );
}
