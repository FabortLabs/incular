import 'package:flutter/foundation.dart';
import 'package:flutter/rendering.dart' show ScrollDirection;
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('a live user scroll postpones handwriting bounds until idle', (
    tester,
  ) async {
    final editing = TextEditingController.fromValue(
      const TextEditingValue(
        text: 'Text1\nText2\nText3\nText4\nText5\nText6',
        selection: TextSelection.collapsed(offset: 0),
      ),
    );
    final focus = FocusNode();
    final scroll = ScrollController();
    addTearDown(editing.dispose);
    addTearDown(focus.dispose);
    addTearDown(scroll.dispose);
    final calls = <List<dynamic>>[];
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      SystemChannels.textInput,
      (MethodCall method) async {
        if (method.method == 'TextInput.setSelectionRects') {
          calls.add(method.arguments as List<dynamic>);
        }
        return null;
      },
    );

    await tester.pumpWidget(
      MediaQuery(
        data: const MediaQueryData(),
        child: Directionality(
          textDirection: TextDirection.ltr,
          child: Center(
            child: SizedBox(
              width: 200,
              height: 50,
              child: EditableText(
                controller: editing,
                focusNode: focus,
                scrollController: scroll,
                maxLines: null,
                cursorWidth: 0,
                style: const TextStyle(),
                cursorColor: const Color(0xff000000),
                backgroundCursorColor: const Color(0xff000000),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.showKeyboard(find.byType(EditableText));
    await tester.pumpAndSettle();
    expect(calls, isNotEmpty);
    final initial = calls.last;
    calls.clear();

    final gesture = await tester.startGesture(tester.getCenter(find.byType(EditableText)));
    // Default DragStartBehavior.start consumes the first movement to win.
    await gesture.moveBy(const Offset(0, -20));
    await tester.pump(const Duration(milliseconds: 30));
    await gesture.moveBy(const Offset(0, -30));
    await tester.pump(const Duration(milliseconds: 30));
    expect(scroll.offset, greaterThan(0));
    expect(scroll.position.userScrollDirection, isNot(ScrollDirection.idle));
    expect(calls, isEmpty);

    await gesture.up();
    await tester.pumpAndSettle();
    expect(scroll.position.userScrollDirection, ScrollDirection.idle);
    expect(calls, isNotEmpty);
    expect(calls.last, isNot(equals(initial)));
    expect(calls.last.first[4] as int, greaterThan(initial.first[4] as int));
  }, variant: const TargetPlatformVariant(<TargetPlatform>{TargetPlatform.iOS}));
}
