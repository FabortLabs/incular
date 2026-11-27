import 'dart:ui' show SemanticsAction;

import 'package:flutter/semantics.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('semantic editing uses keyboard cause and select-all preserves toolbar cause', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    final TextEditingController controller = TextEditingController(text: 'abcdef');
    final FocusNode focus = FocusNode();
    final List<(TextSelection, SelectionChangedCause?)> changes = [];
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    controller.selection = const TextSelection.collapsed(offset: 0);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: EditableText(
          controller: controller,
          focusNode: focus,
          autofocus: true,
          style: const TextStyle(fontSize: 14),
          cursorColor: const Color(0xffffffff),
          backgroundCursorColor: const Color(0xff333333),
          onSelectionChanged: (selection, cause) => changes.add((selection, cause)),
        ),
      ),
    );
    await tester.pump();
    expect(focus.hasPrimaryFocus, isTrue);
    changes.clear();

    final Finder editable = find.byType(EditableText);
    final SemanticsOwner owner = tester.binding.pipelineOwner.semanticsOwner!;
    SemanticsNode? editingNode;
    bool findEditingNode(SemanticsNode candidate) {
      if (candidate.getSemanticsData().hasAction(SemanticsAction.setSelection)) {
        editingNode = candidate;
        return false;
      }
      candidate.visitChildren(findEditingNode);
      return editingNode == null;
    }
    findEditingNode(owner.rootSemanticsNode!);
    expect(editingNode, isNotNull);
    final SemanticsNode node = editingNode!;
    owner.performAction(node.id, SemanticsAction.setSelection, <String, int>{'base': 1, 'extent': 3});
    await tester.pump();
    expect(changes.last, (const TextSelection(baseOffset: 1, extentOffset: 3), SelectionChangedCause.keyboard));
    owner.performAction(node.id, SemanticsAction.moveCursorForwardByCharacter, true);
    await tester.pump();
    expect(changes.last, (const TextSelection(baseOffset: 1, extentOffset: 4), SelectionChangedCause.keyboard));
    owner.performAction(node.id, SemanticsAction.setText, 'ok');
    await tester.pump();
    expect(controller.text, 'ok');
    expect(changes.last, (const TextSelection.collapsed(offset: 2), SelectionChangedCause.keyboard));

    tester.state<EditableTextState>(editable).selectAll(SelectionChangedCause.toolbar);
    await tester.pump();
    expect(changes.last, (const TextSelection(baseOffset: 0, extentOffset: 2), SelectionChangedCause.toolbar));
    expect(changes, hasLength(4));
    semantics.dispose();
    expect(focus.hasPrimaryFocus, isTrue);
  });
}
