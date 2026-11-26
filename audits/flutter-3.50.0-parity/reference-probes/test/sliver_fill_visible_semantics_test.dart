import 'package:flutter/semantics.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final (axis, reverse) in [
    (Axis.vertical, false),
    (Axis.horizontal, true),
  ]) {
    testWidgets('SliverFillViewport strict page edges $axis reverse=$reverse', (
      tester,
    ) async {
      final semantics = tester.ensureSemantics();
      try {
        final controller = ScrollController();
        addTearDown(controller.dispose);
        tester.view.devicePixelRatio = 1;
        tester.view.physicalSize = axis == Axis.vertical
            ? const Size(800, 600)
            : const Size(600, 800);
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: CustomScrollView(
              controller: controller,
              scrollDirection: axis,
              reverse: reverse,
              slivers: [
                SliverFillViewport(
                  allowImplicitScrolling: false,
                  delegate: SliverChildListDelegate(
                    const [Text('Page 1'), Text('Page 2'), Text('Page 3')],
                    addAutomaticKeepAlives: false,
                    addSemanticIndexes: false,
                  ),
                ),
              ],
            ),
          ),
        );

        List<String> labels() {
          final result = <String>[];
          void walk(SemanticsNode node) {
            final label = node.getSemanticsData().label;
            if (label.startsWith('Page ')) {
              result.add(label);
            }
            node.visitChildren((child) {
              walk(child);
              return true;
            });
          }

          walk(
            tester
                .binding
                .renderViews
                .single
                .owner!
                .semanticsOwner!
                .rootSemanticsNode!,
          );
          return result..sort();
        }

        for (final (offset, expected) in [
          (0.0, ['Page 1']),
          (599.0, ['Page 1', 'Page 2']),
          (600.0, ['Page 2']),
          (601.0, ['Page 2', 'Page 3']),
          (600.0, ['Page 2']),
        ]) {
          controller.jumpTo(offset);
          await tester.pump();
          expect(labels(), expected, reason: 'offset=$offset');
        }
      } finally {
        semantics.dispose();
      }
    });
  }
}
