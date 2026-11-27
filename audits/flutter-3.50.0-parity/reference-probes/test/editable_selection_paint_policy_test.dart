import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('a raw editable paints its valid selection independently of focus', (tester) async {
    const highlight = Color(0xff006f00);
    final controller = TextEditingController(text: 'abc\nabcdefghij');
    final focus = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);

    await tester.pumpWidget(
      MediaQuery(
        data: const MediaQueryData(),
        child: Directionality(
          textDirection: TextDirection.ltr,
          child: Align(
            alignment: Alignment.topLeft,
            child: EditableText(
              controller: controller,
              focusNode: focus,
              maxLines: null,
              style: const TextStyle(fontFamily: 'FlutterTest', fontSize: 10, height: 1),
              cursorColor: const Color(0xff000000),
              backgroundCursorColor: const Color(0xff777777),
              selectionColor: highlight,
            ),
          ),
        ),
      ),
    );

    final render = tester.allRenderObjects.whereType<RenderEditable>().single;
    expect(render.selectionWidthStyle, ui.BoxWidthStyle.tight);
    expect(render.selectionHeightStyle, ui.BoxHeightStyle.includeLineSpacingMiddle);
    expect(focus.hasFocus, isFalse);
    expect(render, isNot(paints..rect(color: highlight)));
    const range = TextSelection(baseOffset: 0, extentOffset: 4);
    const rect = Rect.fromLTRB(0, 0, 30, 10);

    controller.selection = range;
    await tester.pump();
    expect(focus.hasFocus, isFalse);
    expect(render, paints..rect(color: highlight, rect: rect));

    focus.requestFocus();
    await tester.pump();
    controller.selection = range;
    await tester.pump();
    expect(focus.hasFocus, isTrue);
    expect(render, paints..rect(color: highlight, rect: rect));

    focus.unfocus();
    await tester.pump();
    expect(focus.hasFocus, isFalse);
    expect(controller.selection, range);
    expect(render, paints..rect(color: highlight, rect: rect));

    controller.selection = const TextSelection.collapsed(offset: 4);
    await tester.pump();
    expect(render, isNot(paints..rect(color: highlight)));
  });
}
