import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'leaf detachment clears scope history before the scope takes focus on the next frame',
    (tester) async {
      final key = GlobalKey();
      await tester.pumpWidget(SizedBox(key: key));
      final context = key.currentContext!;
      final scope = FocusScopeNode();
      final parent = FocusNode();
      final leaf = FocusNode();
      final sibling = FocusNode();
      addTearDown(scope.dispose);
      addTearDown(parent.dispose);
      addTearDown(leaf.dispose);
      addTearDown(sibling.dispose);

      scope
          .attach(context)
          .reparent(parent: tester.binding.focusManager.rootScope);
      parent.attach(context).reparent(parent: scope);
      final attachment = leaf.attach(context)..reparent(parent: parent);
      sibling.attach(context).reparent(parent: parent);
      leaf.requestFocus();
      await tester.pump();
      expect(scope.focusedChild, same(leaf));
      attachment.detach();
      expect(scope.focusedChild, isNull);
      expect(scope.hasFocus, isFalse);
      expect(sibling.hasFocus, isFalse);

      await tester.pump();
      expect(scope.hasPrimaryFocus, isTrue);
      expect(scope.focusedChild, isNull);
      expect(sibling.hasFocus, isFalse);
    },
  );
}
