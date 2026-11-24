import 'package:flutter/rendering.dart' show SemanticsNode;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final ensure in [false, true]) {
    testWidgets('off-cache adapter semantics after a jump, ensure=$ensure', (
      tester,
    ) async {
      final handle = tester.ensureSemantics();
      try {
        final controller = ScrollController();
        addTearDown(controller.dispose);
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: Center(
              child: SizedBox(
                height: 200,
                child: CustomScrollView(
                  controller: controller,
                  slivers: List<Widget>.generate(30, (i) {
                    final sliver = SliverToBoxAdapter(
                      child: SizedBox(height: 200, child: Text('Item $i')),
                    );
                    return ensure && i == 4
                        ? SliverEnsureSemantics(sliver: sliver)
                        : sliver;
                  }),
                ),
              ),
            ),
          ),
        );
        // Match the normalized Incular setup without depending on how each
        // framework handles the initial scroll offset.
        controller.jumpTo(300);
        await tester.pump();
        expect(controller.offset, 300);
        expect(controller.position.viewportDimension, 200);

        final nodes = <String, List<SemanticsNode>>{};
        void collect(SemanticsNode node) {
          final label = node.getSemanticsData().label;
          if (label.startsWith('Item ')) {
            (nodes[label] ??= <SemanticsNode>[]).add(node);
          }
          node.visitChildren((child) {
            collect(child);
            return true;
          });
        }

        collect(
          tester
              .binding
              .renderViews
              .single
              .owner!
              .semanticsOwner!
              .rootSemanticsNode!,
        );
        for (var i = 0; i < 4; i++) {
          final node = nodes.remove('Item $i')!.single;
          expect(
            node.getSemanticsData().flagsCollection.isHidden,
            i == 0 || i == 3,
          );
        }
        expect(nodes.keys, ensure ? ['Item 4'] : isEmpty);
        if (ensure) {
          expect(nodes['Item 4'], hasLength(1));
        }
      } finally {
        handle.dispose();
      }
    });
  }
}
