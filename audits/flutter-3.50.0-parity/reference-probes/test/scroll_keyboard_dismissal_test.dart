import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('wheel, jump and unrelated scopes keep the editor focused', (
    tester,
  ) async {
    final text = TextEditingController(text: 'still editing');
    final focus = FocusNode();
    final scroll = ScrollController();
    addTearDown(text.dispose);
    addTearDown(focus.dispose);
    addTearDown(scroll.dispose);

    const scrollKey = ValueKey('scroll');
    const onDrag = ScrollViewKeyboardDismissBehavior.onDrag;
    const manual = ScrollViewKeyboardDismissBehavior.manual;

    Future<void> layout({
      required bool separateScope,
      required ScrollViewKeyboardDismissBehavior ambient,
      ScrollViewKeyboardDismissBehavior? override,
    }) async {
      final editor = SizedBox(
        height: 50,
        child: EditableText(
          controller: text,
          focusNode: focus,
          onTapOutside: (_) {},
          style: const TextStyle(fontSize: 14),
          cursorColor: const Color(0xFF000000),
          backgroundCursorColor: const Color(0xFFFFFFFF),
        ),
      );
      const contents = SizedBox(height: 1500, width: 300);
      final viewport = FocusScope(
        child: SingleChildScrollView(
          key: scrollKey,
          controller: scroll,
          keyboardDismissBehavior: override,
          child: separateScope ? contents : Column(children: [editor, contents]),
        ),
      );
      await tester.pumpWidget(
        WidgetsApp(
          color: const Color(0xFFFFFFFF),
          builder: (context, child) => ScrollConfiguration(
            behavior: const ScrollBehavior().copyWith(
              keyboardDismissBehavior: ambient,
            ),
            child: separateScope
                ? Column(
                    children: [
                      FocusScope(child: editor),
                      Expanded(child: viewport),
                    ],
                  )
                : viewport,
          ),
        ),
      );
      focus.requestFocus();
      await tester.pump();
      expect(focus.hasPrimaryFocus, isTrue);
    }

    Future<void> drag() async {
      await tester.drag(find.byKey(scrollKey), const Offset(0, -65));
      await tester.pumpAndSettle();
    }

    await layout(separateScope: false, ambient: onDrag);
    scroll.jumpTo(35);
    await tester.pump();
    final center = tester.getCenter(find.byKey(scrollKey));
    await tester.sendEventToBinding(
      PointerScrollEvent(
        position: center,
        scrollDelta: const Offset(0, 30),
      ),
    );
    await tester.pump();
    expect(focus.hasPrimaryFocus, isTrue);
    await drag();
    expect(focus.hasPrimaryFocus, isFalse);

    await layout(separateScope: false, ambient: onDrag, override: manual);
    await drag();
    expect(focus.hasPrimaryFocus, isTrue);

    await layout(separateScope: true, ambient: manual, override: onDrag);
    await drag();
    expect(focus.hasPrimaryFocus, isTrue);
  });
}
