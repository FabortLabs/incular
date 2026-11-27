import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('ordinary chords require their exact held set on down and repeat', (tester) async {
    int chords = 0;
    int singles = 0;
    const a = LogicalKeyboardKey.keyA;
    const b = LogicalKeyboardKey.keyB;
    const c = LogicalKeyboardKey.keyC;
    await tester.pumpWidget(
      CallbackShortcuts(
        bindings: {
          LogicalKeySet(a, b): () => chords += 1,
          const SingleActivator(b): () => singles += 1,
        },
        child: const Focus(autofocus: true, child: SizedBox()),
      ),
    );
    await tester.pump();

    await tester.sendKeyDownEvent(a);
    expect((chords, singles), (0, 0));
    await tester.sendKeyDownEvent(b);
    expect((chords, singles), (1, 1));
    await tester.sendKeyRepeatEvent(b);
    expect((chords, singles), (2, 2));
    await tester.sendKeyUpEvent(b);
    await tester.sendKeyUpEvent(a);
    expect((chords, singles), (2, 2));

    await tester.sendKeyDownEvent(b);
    expect((chords, singles), (2, 3));
    await tester.sendKeyDownEvent(a);
    expect((chords, singles), (3, 3));
    await tester.sendKeyUpEvent(b);
    await tester.sendKeyUpEvent(a);
    expect((chords, singles), (3, 3));

    await tester.sendKeyDownEvent(c);
    await tester.sendKeyDownEvent(a);
    await tester.sendKeyDownEvent(b);
    expect((chords, singles), (3, 4));
    await tester.sendKeyRepeatEvent(b);
    expect((chords, singles), (3, 5));
    await tester.sendKeyUpEvent(c);
    expect((chords, singles), (3, 5));
    await tester.sendKeyRepeatEvent(b);
    expect((chords, singles), (4, 6));
    await tester.sendKeyUpEvent(b);
    await tester.sendKeyUpEvent(a);
    expect((chords, singles), (4, 6));
    expect(HardwareKeyboard.instance.logicalKeysPressed, isEmpty);
  });
}
