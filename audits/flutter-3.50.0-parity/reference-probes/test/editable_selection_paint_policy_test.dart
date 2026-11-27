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
  testWidgets('a width style update changes only the highlight on the retained editable', (tester) async {
    const highlight = Color(0xff006f00);
    final controller = TextEditingController(text: 'abc\nabcdefghij')
      ..selection = const TextSelection(baseOffset: 0, extentOffset: 4);
    final focus = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);

    Widget build(ui.BoxWidthStyle width) => MediaQuery(
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
            selectionWidthStyle: width,
          ),
        ),
      ),
    );

    await tester.pumpWidget(build(ui.BoxWidthStyle.tight));
    final render = tester.allRenderObjects.whereType<RenderEditable>().single;
    final originalSize = render.size;
    const first = Rect.fromLTRB(0, 0, 30, 10);
    const extension = Rect.fromLTRB(30, 0, 100, 10);
    expect(render, paints..rect(color: highlight, rect: first));
    expect(render, isNot(paints..rect(color: highlight, rect: extension)));

    await tester.pumpWidget(build(ui.BoxWidthStyle.max));
    expect(tester.allRenderObjects.whereType<RenderEditable>().single, same(render));
    expect(render.size, originalSize);
    expect(render.selectionWidthStyle, ui.BoxWidthStyle.max);
    expect(render, paints
      ..rect(color: highlight, rect: first)
      ..rect(color: highlight, rect: extension));
    expect(focus.hasFocus, isFalse);
  });
  testWidgets('selection height uses real document edges across hard breaks', (tester) async {
    const highlight = Color(0xff006f00);
    final controller = TextEditingController(text: 'ab\ncd\nef')
      ..selection = const TextSelection(baseOffset: 0, extentOffset: 8);
    final focus = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);

    Widget build(ui.BoxHeightStyle? height) => MediaQuery(
      data: const MediaQueryData(),
      child: Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: EditableText(
            controller: controller,
            focusNode: focus,
            maxLines: null,
            style: const TextStyle(fontFamily: 'FlutterTest', fontSize: 10, height: 2),
            cursorColor: const Color(0xff000000),
            backgroundCursorColor: const Color(0xff777777),
            selectionColor: highlight,
            selectionHeightStyle: height,
          ),
        ),
      ),
    );
    await tester.pumpWidget(build(null));
    final render = tester.allRenderObjects.whereType<RenderEditable>().single;
    expect(render.selectionHeightStyle, ui.BoxHeightStyle.includeLineSpacingMiddle);
    expect(render, paints
      ..rect(color: highlight, rect: const Rect.fromLTRB(0, 7.5, 20, 23.75))
      ..rect(color: highlight, rect: const Rect.fromLTRB(0, 23.75, 20, 43.75))
      ..rect(color: highlight, rect: const Rect.fromLTRB(0, 43.75, 20, 60)));

    await tester.pumpWidget(build(ui.BoxHeightStyle.max));
    expect(tester.allRenderObjects.whereType<RenderEditable>().single, same(render));
    expect(render, paints
      ..rect(color: highlight, rect: const Rect.fromLTRB(0, 0, 20, 20))
      ..rect(color: highlight, rect: const Rect.fromLTRB(0, 20, 20, 40))
      ..rect(color: highlight, rect: const Rect.fromLTRB(0, 40, 20, 60)));
  });
}
