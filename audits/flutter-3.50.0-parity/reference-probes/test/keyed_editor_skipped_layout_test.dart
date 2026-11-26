import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('a keyed focused editor keeps its state across a preceding row', (
    tester,
  ) async {
    final controller = TextEditingController(text: 'draft');
    final focus = FocusNode();
    final list = ScrollController();
    final key = GlobalKey<EditableTextState>();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    addTearDown(list.dispose);
    Widget build({required bool prepend}) => MediaQuery(
      data: const MediaQueryData(),
      child: Directionality(
        textDirection: TextDirection.ltr,
        child: ListView(
          controller: list,
          children: <Widget>[
            if (prepend) const SizedBox(height: 6000),
            EditableText(
              key: key,
              controller: controller,
              focusNode: focus,
              autofocus: true,
              maxLines: null,
              style: const TextStyle(fontSize: 14),
              textAlign: prepend ? TextAlign.right : TextAlign.left,
              cursorColor: const Color(0xff000000),
              backgroundCursorColor: const Color(0xff000000),
              showCursor: prepend,
            ),
          ],
        ),
      ),
    );

    await tester.pumpWidget(build(prepend: false));
    await tester.pump();
    final state = key.currentState!;
    expect(focus.hasPrimaryFocus, isTrue);
    await tester.pumpWidget(build(prepend: true));
    expect(tester.takeException(), isNull);
    expect(key.currentState, same(state));
    expect(focus.hasPrimaryFocus, isTrue);
    expect(list.offset, 0);
    expect(controller.text, 'draft');
  });
}
