import 'dart:ui' show SemanticsAction;

import 'package:flutter/semantics.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final bool nested in [false, true]) {
    testWidgets(
      'semantic point action follows inverse paint order (nested: $nested)',
      (tester) async {
        tester.view.devicePixelRatio = 1.0;
        tester.view.physicalSize = const Size(200, 200);
        addTearDown(tester.view.resetDevicePixelRatio);
        addTearDown(tester.view.resetPhysicalSize);
        const container = ValueKey<String>('semantic container');
        final trace = <String>[];
        Widget scene(
          double bottomOrder,
          double topOrder,
          bool topTap,
          bool swapPaint,
        ) {
          Widget node(String label, double order, bool tap) => Semantics(
            key: ValueKey<String>(label),
            container: true,
            sortKey: OrdinalSortKey(order),
            label: label,
            onTap: tap ? () => trace.add(label) : null,
            child: const SizedBox.expand(),
          );
          final List<Widget> children = [
            node('painted first', bottomOrder, true),
            node('painted last', topOrder, topTap),
          ];
          final stack = Stack(
            children: swapPaint ? children.reversed.toList() : children,
          );
          return Directionality(
            textDirection: TextDirection.ltr,
            child: nested
                ? Semantics(
                    key: container,
                    container: true,
                    explicitChildNodes: true,
                    label: 'container',
                    child: stack,
                  )
                : stack,
          );
        }

        for (final (
              double bottom,
              double top,
              bool tap,
              bool swapPaint,
              String expected,
            )
            in [
              (10.0, 0.0, true, false, 'painted last'),
              (0.0, 10.0, true, false, 'painted last'),
              (10.0, 0.0, false, false, 'painted first'),
              (10.0, 0.0, true, false, 'painted last'),
              (10.0, 0.0, true, true, 'painted first'),
              (10.0, 0.0, true, false, 'painted last'),
            ]) {
          await tester.pumpWidget(scene(bottom, top, tap, swapPaint));
          final SemanticsOwner owner =
              tester.binding.pipelineOwner.semanticsOwner!;
          final SemanticsNode parent = nested
              ? tester.getSemantics(find.byKey(container))
              : owner.rootSemanticsNode!;
          final children = parent.debugListChildrenInOrder(
            DebugSemanticsDumpOrder.traversalOrder,
          );
          expect(
            children.map((node) => node.label),
            bottom < top
                ? ['painted first', 'painted last']
                : ['painted last', 'painted first'],
          );
          owner.performActionAt(const Offset(100, 100), SemanticsAction.tap);
          expect(trace, [expected]);
          trace.clear();
          owner.performActionAt(const Offset(200, 100), SemanticsAction.tap);
          expect(trace, isEmpty);
        }
      },
    );
  }
}
