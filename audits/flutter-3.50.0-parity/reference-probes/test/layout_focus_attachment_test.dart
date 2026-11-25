import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'a request before attachment beats autofocus for an editor built in layout',
    (tester) async {
      final node = FocusNode();
      final other = FocusNode();
      final controller = TextEditingController(text: 'Text');
      addTearDown(node.dispose);
      addTearDown(other.dispose);
      addTearDown(controller.dispose);
      node.requestFocus();
      expect(node.hasFocus, isFalse);
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Column(
            children: [
              LayoutBuilder(
                builder: (_, _) => Focus(
                  focusNode: other,
                  autofocus: true,
                  child: const SizedBox(height: 20),
                ),
              ),
              LayoutBuilder(
                builder: (_, _) => EditableText(
                  controller: controller,
                  focusNode: node,
                  style: const TextStyle(
                    fontSize: 14,
                    color: Color(0xff000000),
                  ),
                  cursorColor: const Color(0xff000000),
                  backgroundCursorColor: const Color(0xff808080),
                ),
              ),
            ],
          ),
        ),
      );
      expect(node.hasPrimaryFocus, isTrue);
      expect(other.hasFocus, isFalse);
      expect(
        controller.selection,
        const TextSelection(baseOffset: 0, extentOffset: 4),
      );
      await tester.pump();
      expect(node.hasPrimaryFocus, isTrue);
    },
    variant: const TargetPlatformVariant({TargetPlatform.linux}),
  );
}
