import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final Axis axis in Axis.values) {
    for (final bool reverse in <bool>[false, true]) {
      testWidgets(
        'Pinned header overlap hides rows at the exact sliver clip edge ($axis, reverse=$reverse)',
        (WidgetTester tester) async {
          const ValueKey<String> headerKey = ValueKey<String>('pinned');
          const List<ValueKey<int>> keys = <ValueKey<int>>[
            ValueKey<int>(0),
            ValueKey<int>(1),
            ValueKey<int>(2),
          ];
          final ScrollController controller = ScrollController();
          addTearDown(controller.dispose);
          await tester.pumpWidget(
            Directionality(
              textDirection: TextDirection.ltr,
              child: CustomScrollView(
                controller: controller,
                scrollDirection: axis,
                reverse: reverse,
                semanticChildCount: 50,
                slivers: <Widget>[
                  PinnedHeaderSliver(
                    child: Semantics(
                      key: headerKey,
                      label: 'pinned',
                      container: true,
                      child: const SizedBox(width: 20, height: 20),
                    ),
                  ),
                  SliverFixedExtentList.builder(
                    itemExtent: 50,
                    itemCount: 50,
                    itemBuilder: (BuildContext context, int index) => Semantics(
                      key: index < keys.length ? keys[index] : null,
                      label: 'row $index',
                      container: true,
                      child: const SizedBox(width: 50, height: 50),
                    ),
                  ),
                ],
              ),
            ),
          );

          SemanticsNode indexedRow(int i) {
            SemanticsNode row = tester.getSemantics(
              find.byKey(keys[i], skipOffstage: false),
            );
            while (row.indexInParent == null && row.parent != null) {
              row = row.parent!;
            }
            expect(row.indexInParent, i);
            return row;
          }

          final SemanticsNode first = indexedRow(0);
          final SemanticsNode pane = first.parent!;
          final SemanticsNode outer = pane.parent!;
          final int innerId = pane.id;
          final int firstId = first.id;
          expect(pane.getSemanticsData().scrollChildCount, 50);
          expect(pane.getSemanticsData().scrollIndex, 0);
          expect(outer.getSemanticsData().scrollChildCount, isNull);
          expect(outer.getSemanticsData().scrollIndex, isNull);

          for (final (double offset, int index) in <(double, int)>[
            (49, 0),
            (50, 1),
            (55, 1),
            (70, 1),
          ]) {
            controller.jumpTo(offset);
            await tester.pump();
            final SemanticsNode firstRow = indexedRow(0);
            final SemanticsNode header = tester.getSemantics(
              find.byKey(headerKey),
            );
            final SemanticsNode current = indexedRow(1).parent!;
            expect(current.id, innerId);
            expect(firstRow.id, firstId);
            expect(
              firstRow.getSemanticsData().flagsCollection.isHidden,
              index > 0,
              reason: 'scroll offset $offset',
            );
            expect(current.getSemanticsData().scrollChildCount, 50);
            expect(
              current.getSemanticsData().scrollIndex,
              index,
              reason: 'scroll offset $offset',
            );
            expect(
              (header.isTagged(RenderViewport.excludeFromScrolling)
                      ? header
                      : header.parent!)
                  .isTagged(RenderViewport.excludeFromScrolling),
              isTrue,
            );
            expect(current.parent?.id, outer.id);
          }
        },
      );
    }
  }
}
