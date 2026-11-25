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
  testWidgets(
    'semantic tap notifications are ordered and cannot consume the target',
    (tester) async {
      final handle = tester.ensureSemantics();
      try {
        final trace = <String>[];
        Offset? position;
        const targetKey = ValueKey<String>('action target');
        const box = SizedBox(
          width: 100,
          height: 50,
          child: ColoredBox(color: Color(0xffffffff)),
        );
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: Listener(
              onPointerDown: (_) => trace.add('unexpected raw down'),
              child: TapRegionSurface(
                child: Row(
                  children: [
                    TapRegion(
                      consumeOutsideTaps: true,
                      onTapOutside: (event) {
                        trace.add('A.outside');
                        position = event.position;
                      },
                      onTapUpOutside: (_) => trace.add('unexpected up'),
                      child: box,
                    ),
                    TapRegion(
                      behavior: HitTestBehavior.opaque,
                      onTapInside: (_) => trace.add('B.inside'),
                      child: Semantics(
                        key: targetKey,
                        label: 'action target',
                        onTap: () => trace.add('target'),
                        onLongPress: () => trace.add('target'),
                        child: box,
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        );
        final node = tester.semantics.find(
          find.bySemanticsLabel('action target'),
        );
        for (final action in [SemanticsAction.tap, SemanticsAction.longPress]) {
          trace.clear();
          position = null;
          tester.binding.platformDispatcher.onSemanticsActionEvent!(
            SemanticsActionEvent(
              type: action,
              nodeId: node.id,
              viewId: tester.view.viewId,
            ),
          );
          await tester.pump();
          expect(trace, ['A.outside', 'B.inside', 'target']);
          expect(position, tester.getCenter(find.byKey(targetKey)));
        }
      } finally {
        handle.dispose();
      }
    },
  );
}
