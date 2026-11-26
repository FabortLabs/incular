import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Indexed scrolling pane counts only visible indexed rows', (
    WidgetTester tester,
  ) async {
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
          semanticChildCount: 50,
          slivers: <Widget>[
            PinnedHeaderSliver(
              child: Semantics(
                key: headerKey,
                label: 'pinned',
                container: true,
                child: const SizedBox(height: 20),
              ),
            ),
            SliverFixedExtentList.builder(
              itemExtent: 50,
              itemCount: 50,
              itemBuilder: (BuildContext context, int index) => Semantics(
                key: index < keys.length ? keys[index] : null,
                label: 'row $index',
                container: true,
                child: const SizedBox(height: 50),
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

    controller.jumpTo(70);
    await tester.pump();
    final SemanticsNode hidden = indexedRow(0);
    final SemanticsNode second = indexedRow(1);
    final SemanticsNode header = tester.getSemantics(find.byKey(headerKey));
    final SemanticsNode current = second.parent!;
    expect(current.id, innerId);
    expect(hidden.id, firstId);
    expect(hidden.getSemanticsData().flagsCollection.isHidden, isTrue);
    expect(current.getSemanticsData().scrollChildCount, 50);
    expect(current.getSemanticsData().scrollIndex, 1);
    expect(
      (header.isTagged(RenderViewport.excludeFromScrolling)
              ? header
              : header.parent!)
          .isTagged(RenderViewport.excludeFromScrolling),
      isTrue,
    );
    expect(current.parent?.id, outer.id);
    expect(current.parent!.getSemanticsData().scrollChildCount, isNull);
    expect(current.parent!.getSemanticsData().scrollIndex, isNull);
  });

  testWidgets('A grid scroll index counts individual tiles', (
    WidgetTester tester,
  ) async {
    final ScrollController controller = ScrollController();
    addTearDown(controller.dispose);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: GridView.builder(
          controller: controller,
          gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
            crossAxisCount: 3,
          ),
          itemCount: 31,
          itemBuilder: (BuildContext context, int index) =>
              Semantics(label: 'tile $index', child: const SizedBox.expand()),
        ),
      ),
    );
    final SemanticsOwner owner = tester.binding.pipelineOwner.semanticsOwner!;
    SemanticsNode? pane(SemanticsNode node) {
      if (node.getSemanticsData().scrollChildCount != null) {
        return node;
      }
      SemanticsNode? found;
      node.visitChildren((SemanticsNode child) {
        found = pane(child);
        return found == null;
      });
      return found;
    }

    final SemanticsNode initial = pane(owner.rootSemanticsNode!)!;
    expect(initial.getSemanticsData().scrollChildCount, 31);
    expect(initial.getSemanticsData().scrollIndex, 0);
    controller.jumpTo(300);
    await tester.pump();
    final SemanticsNode updated = pane(owner.rootSemanticsNode!)!;
    expect(updated.id, initial.id);
    expect(updated.getSemanticsData().scrollChildCount, 31);
    expect(updated.getSemanticsData().scrollIndex, 3);
  });
}
