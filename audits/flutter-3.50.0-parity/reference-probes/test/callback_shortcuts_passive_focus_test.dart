import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('callback shortcuts expose a passive focus ancestor', (tester) async {
    final leaf = GlobalKey();
    int hits = 0;
    const keyA = LogicalKeyboardKey.keyA;
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: CallbackShortcuts(
          bindings: {LogicalKeySet(keyA): () => hits += 1},
          child: Column(
            children: [
              SizedBox(key: leaf),
              const Focus(autofocus: true, child: SizedBox()),
            ],
          ),
        ),
      ),
    );
    await tester.pump();
    final boundary = Focus.of(leaf.currentContext!);
    expect(boundary.canRequestFocus, isFalse);
    expect(boundary.skipTraversal, isTrue);
    expect(boundary.hasFocus, isTrue);
    expect(boundary.hasPrimaryFocus, isFalse);
    expect(await tester.sendKeyDownEvent(keyA), isTrue);
    expect(await tester.sendKeyUpEvent(keyA), isFalse);
    expect(hits, 1);
  });
}
