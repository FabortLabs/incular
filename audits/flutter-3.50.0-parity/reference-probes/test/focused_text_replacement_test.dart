import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'desktop replacement repairs an unset selection only while focused',
    (tester) async {
      final controller = TextEditingController(text: 'Text');
      final node = FocusNode();
      addTearDown(controller.dispose);
      addTearDown(node.dispose);
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: EditableText(
            controller: controller,
            focusNode: node,
            style: const TextStyle(fontSize: 14, color: Color(0xff000000)),
            cursorColor: const Color(0xff000000),
            backgroundCursorColor: const Color(0xff808080),
          ),
        ),
      );
      node.requestFocus();
      await tester.pump();
      expect(node.hasPrimaryFocus, isTrue);
      controller.text = 'Updated';
      expect(
        controller.selection,
        const TextSelection(baseOffset: 0, extentOffset: 7),
      );
      await tester.pumpWidget(const SizedBox());
      expect(node.hasFocus, isFalse);
      controller.text = 'Off stage';
      expect(controller.selection, const TextSelection.collapsed(offset: -1));
    },
    variant: const TargetPlatformVariant({TargetPlatform.linux}),
  );
}
