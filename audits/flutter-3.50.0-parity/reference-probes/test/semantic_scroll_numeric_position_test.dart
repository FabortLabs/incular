import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final bool sliver in <bool>[false, true]) {
    testWidgets('Scroll position is not spoken value text (sliver=$sliver)', (
      WidgetTester tester,
    ) async {
      final ScrollController controller = ScrollController();
      addTearDown(controller.dispose);
      final Widget child = Semantics(
        label: 'body',
        child: const SizedBox(height: 1600),
      );
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: sliver
              ? CustomScrollView(
                  controller: controller,
                  slivers: <Widget>[SliverToBoxAdapter(child: child)],
                )
              : SingleChildScrollView(controller: controller, child: child),
        ),
      );
      final SemanticsOwner owner = tester.binding.pipelineOwner.semanticsOwner!;
      SemanticsNode? scroller(SemanticsNode node) {
        if ((node.getSemanticsData().scrollExtentMax ?? 0) > 0) {
          return node;
        }
        SemanticsNode? found;
        node.visitChildren((SemanticsNode child) {
          found = scroller(child);
          return found == null;
        });
        return found;
      }

      final id = scroller(owner.rootSemanticsNode!)!.id;
      for (final double offset in <double>[0, 220.25, 220.5, 0]) {
        controller.jumpTo(offset);
        await tester.pump();
        final SemanticsNode node = scroller(owner.rootSemanticsNode!)!;
        final SemanticsData data = node.getSemanticsData();
        expect(node.id, id);
        expect(data.scrollPosition, offset);
        expect(data.scrollExtentMax, 1000);
        expect(data.value, isEmpty);
      }
    });
  }
}
