import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Delegate implements TextSelectionGestureDetectorBuilderDelegate {
  @override
  final editableTextKey = GlobalKey<EditableTextState>();
  @override
  bool get forcePressEnabled => false;
  @override
  bool get selectionEnabled => true;
}

void main() {
  testWidgets('a double drag keeps its original word when the editor scrolls', (
    tester,
  ) async {
    final text = List<String>.generate(
      45,
      (i) => '${i.toString().padLeft(2, '0')} alpha beta gamma',
    ).join('\n');
    final controller = TextEditingController(text: text);
    final focus = FocusNode();
    final scroll = ScrollController();
    final delegate = _Delegate();
    final builder = TextSelectionGestureDetectorBuilder(delegate: delegate);
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    addTearDown(scroll.dispose);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: 400,
            height: 450,
            child: builder.buildGestureDetector(
              behavior: HitTestBehavior.translucent,
              child: EditableText(
                key: delegate.editableTextKey,
                focusNode: focus,
                controller: controller,
                rendererIgnoresPointer: true,
                maxLines: null,
                scrollController: scroll,
                style: const TextStyle(fontSize: 14),
                cursorColor: const Color(0xffffffff),
                backgroundCursorColor: const Color(0xff222222),
              ),
            ),
          ),
        ),
      ),
    );
    final RenderEditable render =
        delegate.editableTextKey.currentState!.renderEditable;
    const origin = Offset(82, 200);
    const first = Offset(224, 200);
    const last = Offset(224, 400);
    TextRange wordAt(Offset point) =>
        render.getWordBoundary(render.getPositionForPoint(point));
    scroll.jumpTo(20);
    await tester.pump();
    final int anchor = wordAt(origin).start;
    final tap = await tester.startGesture(
      origin,
      kind: PointerDeviceKind.mouse,
    );
    await tap.up();
    await tester.pump();

    final drag = await tester.startGesture(
      origin,
      kind: PointerDeviceKind.mouse,
    );
    await drag.moveTo(first);
    await tester.pump();
    expect(controller.selection.baseOffset, anchor);
    expect(controller.selection.extentOffset, wordAt(first).end);
    scroll.jumpTo(150);
    await tester.pump();
    expect(wordAt(origin - const Offset(0, 130)).start, anchor);
    expect(wordAt(origin).start, isNot(anchor));
    final int extent = wordAt(last).end;
    await drag.moveTo(last);
    await tester.pump();
    await drag.up();
    await tester.pump();
    expect(
      controller.selection,
      TextSelection(baseOffset: anchor, extentOffset: extent),
    );
  });
}
