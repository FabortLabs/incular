import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('a dormant scope navigates from its remembered child', (
    tester,
  ) async {
    final first = FocusNode();
    final second = FocusNode();
    final outside = FocusNode();
    final detached = FocusNode();
    addTearDown(first.dispose);
    addTearDown(second.dispose);
    addTearDown(outside.dispose);
    addTearDown(detached.dispose);

    Widget target(FocusNode node) =>
        Focus(focusNode: node, child: const SizedBox(width: 30, height: 30));

    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Column(
          children: [
            FocusScope(
              child: Column(children: [target(first), target(second)]),
            ),
            target(outside),
          ],
        ),
      ),
    );
    first.requestFocus();
    await tester.pump();
    expect(first.hasPrimaryFocus, true);
    outside.requestFocus();
    await tester.pump();
    expect(outside.hasPrimaryFocus, true);

    expect(first.nextFocus(), true);
    await tester.pump();
    expect(second.hasPrimaryFocus, true);
    expect(outside.hasFocus, false);
  });
}
