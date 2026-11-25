import 'dart:typed_data';
import 'dart:ui' show SemanticsAction;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final axis in Axis.values) {
    for (final reverse in [false, true]) {
      testWidgets('semantic scroll directions $axis reverse=$reverse', (
        WidgetTester tester,
      ) async {
        final controller = ScrollController();
        addTearDown(controller.dispose);
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: ListView(
              controller: controller,
              scrollDirection: axis,
              reverse: reverse,
              itemExtent: 40,
              children: List<Widget>.generate(
                60,
                (_) => const SizedBox.square(dimension: 40),
              ),
            ),
          ),
        );
        final owner = tester.binding.pipelineOwner.semanticsOwner!;
        SemanticsNode? locate(SemanticsNode node) {
          if (node.getSemanticsData().hasAction(
            SemanticsAction.scrollToOffset,
          )) {
            return node;
          }
          SemanticsNode? found;
          node.visitChildren((child) {
            found = locate(child);
            return found == null;
          });
          return found;
        }

        final id = locate(owner.rootSemanticsNode!)!.id;
        final (forward, backward) = switch ((axis, reverse)) {
          (Axis.vertical, false) => (
            SemanticsAction.scrollUp,
            SemanticsAction.scrollDown,
          ),
          (Axis.vertical, true) => (
            SemanticsAction.scrollDown,
            SemanticsAction.scrollUp,
          ),
          (Axis.horizontal, false) => (
            SemanticsAction.scrollLeft,
            SemanticsAction.scrollRight,
          ),
          (Axis.horizontal, true) => (
            SemanticsAction.scrollRight,
            SemanticsAction.scrollLeft,
          ),
        };
        void check({required bool canForward, required bool canBackward}) {
          final data = locate(owner.rootSemanticsNode!)!.getSemanticsData();
          expect(data.hasAction(forward), canForward);
          expect(data.hasAction(backward), canBackward);
        }

        check(canForward: true, canBackward: false);
        owner.performAction(id, forward);
        await tester.pumpAndSettle();
        expect(controller.offset, axis == Axis.vertical ? 480.0 : 640.0);
        check(canForward: true, canBackward: true);
        owner.performAction(id, backward);
        await tester.pumpAndSettle();
        expect(controller.offset, 0);
        check(canForward: true, canBackward: false);
        owner.performAction(
          id,
          SemanticsAction.scrollToOffset,
          Float64List.fromList([123.25, 456.25]),
        );
        await tester.pumpAndSettle();
        expect(controller.offset, axis == Axis.vertical ? 456.25 : 123.25);
        controller.jumpTo(controller.position.maxScrollExtent);
        await tester.pumpAndSettle();
        check(canForward: false, canBackward: true);
      });
    }
  }
  testWidgets('horizontal edge fixture has an 840px text content extent', (
    WidgetTester tester,
  ) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: ListView.builder(
          scrollDirection: Axis.horizontal,
          controller: controller,
          itemCount: 10,
          itemBuilder: (context, index) =>
              SizedBox(height: 150, child: Text('Tile $index')),
        ),
      ),
    );
    expect(tester.getSize(find.text('Tile 0')).width, 84);
    expect(controller.position.maxScrollExtent, 40);
  });
}
