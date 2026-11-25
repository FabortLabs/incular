import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('default scope nodes retain their identity and focus history', (
    WidgetTester tester,
  ) async {
    final first = FocusNode();
    final second = FocusNode();
    addTearDown(first.dispose);
    addTearDown(second.dispose);
    final keyA = GlobalKey();
    final keyB = GlobalKey();

    Widget scene() => Directionality(
      textDirection: TextDirection.ltr,
      child: FocusScope(
        child: Column(
          children: <Widget>[
            FocusScope(
              child: Focus(
                focusNode: first,
                child: SizedBox(key: keyA),
              ),
            ),
            FocusScope(
              child: Focus(
                focusNode: second,
                child: SizedBox(key: keyB),
              ),
            ),
          ],
        ),
      ),
    );

    await tester.pumpWidget(scene());
    final firstScope = FocusScope.of(keyA.currentContext!);
    final secondScope = FocusScope.of(keyB.currentContext!);
    expect(firstScope, isNot(same(secondScope)));
    expect(firstScope.enclosingScope, same(secondScope.enclosingScope));
    first.requestFocus();
    await tester.pump();
    expect(first.hasPrimaryFocus, isTrue);
    second.requestFocus();
    await tester.pump();
    expect(second.hasPrimaryFocus, isTrue);
    expect(firstScope.focusedChild, same(first));

    await tester.pumpWidget(scene());
    expect(FocusScope.of(keyA.currentContext!), same(firstScope));
    expect(FocusScope.of(keyB.currentContext!), same(secondScope));
    expect(second.hasPrimaryFocus, isTrue);

    firstScope.requestFocus();
    await tester.pump();
    expect(first.hasPrimaryFocus, isTrue);
    expect(second.hasFocus, isFalse);
  });
}
