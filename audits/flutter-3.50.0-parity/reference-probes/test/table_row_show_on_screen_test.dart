import 'dart:ui' show SemanticsAction;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('retained row reveal uses the first registered row rect', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    final controller = ScrollController();
    addTearDown(controller.dispose);

    Widget scene(double firstHeight, Key tableKey) => Directionality(
      textDirection: TextDirection.ltr,
      child: Center(
        child: SizedBox(
          width: 200,
          height: 120,
          child: SingleChildScrollView(
            controller: controller,
            child: Table(
              key: tableKey,
              children: [
                TableRow(
                  children: [
                    SizedBox(height: firstHeight, child: const Text('first')),
                  ],
                ),
                const TableRow(
                  children: [SizedBox(height: 40, child: Text('second'))],
                ),
              ],
            ),
          ),
        ),
      ),
    );

    SemanticsNode secondRow() {
      final table = tester.getSemantics(find.byType(Table));
      final rows = <SemanticsNode>[];
      table.visitChildren((child) {
        if (child.role == SemanticsRole.row) {
          rows.add(child);
        }
        return true;
      });
      return rows.singleWhere((row) {
        var found = false;
        void findLabel(SemanticsNode node) {
          if (node.label == 'second') {
            found = true;
          }
          node.visitChildren((child) {
            findLabel(child);
            return true;
          });
        }

        findLabel(row);
        return found;
      });
    }

    Future<void> reveal(SemanticsNode row) async {
      tester.binding.pipelineOwner.semanticsOwner!.performAction(
        row.id,
        SemanticsAction.showOnScreen,
      );
      await tester.pumpAndSettle();
    }

    const retained = ValueKey('retained');
    await tester.pumpWidget(scene(50, retained));
    final initial = secondRow().id;
    final before = tester.getTopLeft(find.text('second')).dy;
    await tester.pumpWidget(scene(100, retained));
    expect(secondRow().id, initial);
    expect(tester.getTopLeft(find.text('second')).dy - before, 50);
    expect(controller.position.maxScrollExtent, 20);
    await reveal(secondRow());
    expect(controller.offset, 0);

    await tester.pumpWidget(scene(100, const ValueKey('fresh')));
    expect(secondRow().id, isNot(initial));
    await reveal(secondRow());
    expect(controller.offset, 20);

    semantics.dispose();
  });
}
