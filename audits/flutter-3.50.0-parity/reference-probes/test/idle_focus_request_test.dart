import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'requested focus settles without another input or a semantics annotation',
    (tester) async {
      final node = FocusNode();
      addTearDown(node.dispose);
      await tester.pumpWidget(
        Focus(
          focusNode: node,
          includeSemantics: false,
          child: const SizedBox.shrink(),
        ),
      );
      expect(node.hasPrimaryFocus, isFalse);
      node.requestFocus();
      expect(node.hasPrimaryFocus, isFalse);
      await tester.idle();
      expect(node.hasPrimaryFocus, isTrue);
      node.unfocus();
      await tester.idle();
      expect(node.hasPrimaryFocus, isFalse);
    },
  );
}
