import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('an ignored text renderer keeps focus and delegates pointer selection', (
    WidgetTester tester,
  ) async {
    final TextEditingController controller = TextEditingController(text: 'alpha beta gamma');
    final FocusNode focus = FocusNode();
    final List<SelectionChangedCause?> causes = [];
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    controller.selection = const TextSelection.collapsed(offset: 1);
    final GlobalKey<EditableTextState> key = GlobalKey<EditableTextState>();

    Future<void> mount({required bool ignore}) async {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Align(
            alignment: Alignment.topLeft,
            child: EditableText(
              key: key,
              focusNode: focus,
              controller: controller,
              autofocus: true,
              rendererIgnoresPointer: ignore,
              style: const TextStyle(fontSize: 14),
              cursorColor: const Color(0xffffffff),
              backgroundCursorColor: const Color(0xff222222),
              onSelectionChanged: (selection, cause) => causes.add(cause),
            ),
          ),
        ),
      );
    }

    await mount(ignore: true);
    await tester.pump();
    expect(focus.hasPrimaryFocus, isTrue);
    causes.clear();
    final Offset origin = tester.getTopLeft(find.byType(EditableText));
    final Offset point = origin + const Offset(58, 7);
    final Offset other = origin + const Offset(30, 7);
    final RenderEditable render = key.currentState!.renderEditable;
    expect(render.getPositionForPoint(point).offset, 4);
    await tester.tapAt(point);
    await tester.pump();
    expect(controller.selection.baseOffset, 1);
    expect(causes, isEmpty);
    expect(focus.hasPrimaryFocus, isTrue);

    focus.unfocus();
    await tester.pump();
    await tester.tapAt(point);
    await tester.pump();
    expect(focus.hasPrimaryFocus, isFalse);
    expect(controller.selection.baseOffset, 1);
    focus.requestFocus();
    await tester.pump();
    causes.clear();

    render.selectPositionAt(from: point, cause: SelectionChangedCause.tap);
    await tester.pump();
    expect(controller.selection.baseOffset, 4);
    expect(causes, [SelectionChangedCause.tap]);

    await mount(ignore: false);
    expect(key.currentState!.renderEditable, same(render));
    await tester.tapAt(other);
    await tester.pump();
    expect(controller.selection.baseOffset, 2);
    expect(causes, [SelectionChangedCause.tap, SelectionChangedCause.tap]);
  });
}
