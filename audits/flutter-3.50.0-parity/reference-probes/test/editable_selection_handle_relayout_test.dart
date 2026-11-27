import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('selection endpoints track a same-editor font-size update', (tester) async {
    final controller = TextEditingController(text: 'initial value');
    final focus = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    Widget build(double fontSize) => MediaQuery(
      data: const MediaQueryData(),
      child: Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: EditableText(
            controller: controller,
            focusNode: focus,
            style: TextStyle(fontFamily: 'FlutterTest', fontSize: fontSize),
            cursorColor: const Color(0xff000000),
            backgroundCursorColor: const Color(0xff777777),
          ),
        ),
      ),
    );
    await tester.pumpWidget(build(18));
    final render = tester.allRenderObjects.whereType<RenderEditable>().single;
    render.selectWordsInRange(from: Offset.zero, cause: SelectionChangedCause.longPress);
    await tester.pump();
    expect(controller.selection, const TextSelection(baseOffset: 0, extentOffset: 7));
    final before = render.getEndpointsForSelection(controller.selection);
    expect(before.map((endpoint) => endpoint.point.dx), [0, 126]);

    await tester.pumpWidget(build(30));
    expect(tester.allRenderObjects.whereType<RenderEditable>().single, same(render));
    expect(controller.selection, const TextSelection(baseOffset: 0, extentOffset: 7));
    final after = render.getEndpointsForSelection(controller.selection);
    expect(after.map((endpoint) => endpoint.point.dx), [0, 210]);
    expect(after[0].point.dy - before[0].point.dy, 12);
    expect(after[1].point.dy - before[1].point.dy, 12);
  });
}
