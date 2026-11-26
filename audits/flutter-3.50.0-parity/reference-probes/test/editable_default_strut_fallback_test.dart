import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('The default EditableText strut keeps fallback glyphs from changing line height', (
    WidgetTester tester,
  ) async {
    const fallback = 'Audit Editor Inter';
    await tester.runAsync(() async {
      var directory = Directory.current;
      while (!File('${directory.path}/harness/flutter/fonts/Inter-Regular.ttf').existsSync()) {
        if (directory.parent.path == directory.path) {
          throw StateError('Missing Incular OFL harness font');
        }
        directory = directory.parent;
      }
      final bytes = await File('${directory.path}/harness/flutter/fonts/Inter-Regular.ttf').readAsBytes();
      await (FontLoader(fallback)..addFont(Future.value(ByteData.sublistView(bytes)))).load();
    });

    final controller = TextEditingController(text: 'AΨ');
    final focus = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    Widget scene(StrutStyle? strut) => Directionality(
      textDirection: TextDirection.ltr,
      child: Center(
        child: SizedBox(
          width: 200,
          child: EditableText(
            controller: controller,
            focusNode: focus,
            style: const TextStyle(
              fontFamily: 'Ahem',
              fontFamilyFallback: [fallback],
              fontSize: 20,
            ),
            strutStyle: strut,
            cursorColor: const Color(0),
            backgroundCursorColor: const Color(0),
          ),
        ),
      ),
    );
    final finder = find.byType(EditableText);

    await tester.pumpWidget(scene(null));
    final element = tester.element(finder);
    expect(tester.getSize(finder).height, 20);
    await tester.pumpWidget(scene(StrutStyle.disabled));
    expect(tester.element(finder), same(element));
    expect(tester.getSize(finder).height, greaterThan(20));
    await tester.pumpWidget(scene(null));
    expect(tester.getSize(finder).height, 20);
  });
}
