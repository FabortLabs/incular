import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('the default action explicitly aligns a visible focus target', (
    tester,
  ) async {
    final node = FocusNode();
    final scroll = ScrollController(initialScrollOffset: 450);
    addTearDown(node.dispose);
    addTearDown(scroll.dispose);
    final caller = GlobalKey();
    await tester.pumpWidget(
      WidgetsApp(
        color: const Color(0xff202020),
        builder: (context, child) => SingleChildScrollView(
          controller: scroll,
          child: Column(
            children: [
              const SizedBox(height: 900),
              Focus(
                focusNode: node,
                key: caller,
                child: const SizedBox(height: 60),
              ),
              const SizedBox(height: 400),
            ],
          ),
        ),
      ),
    );
    expect(scroll.offset, 450);
    expect(node.hasFocus, isFalse);
    Actions.invoke(caller.currentContext!, RequestFocusIntent(node));
    await tester.pump();
    expect(scroll.offset, 360);
    expect(node.hasFocus, isTrue);

    node.unfocus();
    scroll.jumpTo(0);
    await tester.pump();
    var calls = 0;
    Actions.invoke(
      caller.currentContext!,
      RequestFocusIntent(
        node,
        requestFocusCallback:
            (target, {alignment, alignmentPolicy, curve, duration}) {
              expect(target, same(node));
              expect([
                alignment,
                alignmentPolicy,
                curve,
                duration,
              ], everyElement(isNull));
              calls += 1;
            },
      ),
    );
    await tester.pump();
    expect(calls, 1);
    expect(scroll.offset, 0);
    expect(node.hasFocus, isFalse);
  });
}
