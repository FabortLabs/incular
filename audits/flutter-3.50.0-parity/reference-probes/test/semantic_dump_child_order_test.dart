import 'package:flutter/rendering.dart';
import 'package:flutter/semantics.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'semantics diagnostics distinguish traversal from inverse hit testing',
    (tester) async {
      final handle = tester.ensureSemantics();
      try {
        const parent = ValueKey('parent');
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: Semantics(
              key: parent,
              container: true,
              explicitChildNodes: true,
              label: 'parent',
              child: Stack(
                fit: StackFit.expand,
                children: [
                  Semantics(
                    container: true,
                    sortKey: const OrdinalSortKey(10),
                    label: 'painted first',
                    child: const SizedBox.expand(),
                  ),
                  Semantics(
                    container: true,
                    sortKey: const OrdinalSortKey(0),
                    label: 'painted last',
                    child: const SizedBox.expand(),
                  ),
                ],
              ),
            ),
          ),
        );
        final render = tester.renderObject<RenderSemanticsAnnotations>(
          find.byKey(parent),
        );
        final node = render.debugSemantics!;
        bool firstPrecedesLast(DebugSemanticsDumpOrder order) {
          final dump = node.toStringDeep(childOrder: order);
          final first = dump.indexOf('painted first');
          final last = dump.indexOf('painted last');
          expect(first, greaterThanOrEqualTo(0), reason: dump);
          expect(last, greaterThanOrEqualTo(0), reason: dump);
          return first < last;
        }

        expect(
          firstPrecedesLast(DebugSemanticsDumpOrder.traversalOrder),
          isFalse,
        );
        expect(
          firstPrecedesLast(DebugSemanticsDumpOrder.inverseHitTest),
          isTrue,
        );
      } finally {
        handle.dispose();
      }
    },
  );
}
