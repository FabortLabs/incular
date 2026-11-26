import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('retained focus does not retry autofocus after it was used', (
    tester,
  ) async {
    final FocusNode node = FocusNode();
    addTearDown(node.dispose);
    Widget view(bool autofocus) =>
        Focus(focusNode: node, autofocus: autofocus, child: const SizedBox());
    await tester.pumpWidget(view(false));
    expect(node.hasPrimaryFocus, isFalse);
    await tester.pumpWidget(view(true));
    expect(node.hasPrimaryFocus, isTrue);
    await tester.pumpWidget(view(false));
    node.unfocus();
    await tester.pump();
    expect(node.hasPrimaryFocus, isFalse);
    await tester.pumpWidget(view(true));
    expect(node.hasPrimaryFocus, isFalse);
  });

  testWidgets(
    'retained autofocus in a scope without a focused child is eligible',
    (tester) async {
      final FocusNode outside = FocusNode();
      final FocusNode inside = FocusNode();
      addTearDown(outside.dispose);
      addTearDown(inside.dispose);

      Widget view(bool autofocus) => Column(
        children: <Widget>[
          Focus(focusNode: outside, autofocus: true, child: const SizedBox()),
          FocusScope(
            child: Focus(
              focusNode: inside,
              autofocus: autofocus,
              child: const SizedBox(),
            ),
          ),
        ],
      );
      await tester.pumpWidget(view(false));
      expect(outside.hasPrimaryFocus, isTrue);
      await tester.pumpWidget(view(true));
      expect(inside.hasPrimaryFocus, isTrue);
      expect(outside.hasPrimaryFocus, isFalse);
    },
  );
}
