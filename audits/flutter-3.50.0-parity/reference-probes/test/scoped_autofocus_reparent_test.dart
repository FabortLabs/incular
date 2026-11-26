import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'scoped autofocus discards a reparented target and keeps the first valid request',
    (tester) async {
      await tester.pumpWidget(const SizedBox.shrink());
      final context = tester.element(find.byType(SizedBox));
      final scope = FocusScopeNode();
      final other = FocusScopeNode();
      final moved = FocusNode();
      final first = FocusNode();
      final later = FocusNode();
      for (final node in [scope, other, moved, first, later]) {
        addTearDown(node.dispose);
      }
      FocusAttachment attach(FocusNode child, FocusNode parent) {
        final attachment = child.attach(context);
        attachment.reparent(parent: parent);
        return attachment;
      }

      final root = tester.binding.focusManager.rootScope;
      attach(scope, root);
      attach(other, root);
      final attachment = attach(moved, scope);
      attach(first, scope);
      attach(later, scope);

      scope.autofocus(moved);
      attachment.reparent(parent: other);
      scope.autofocus(first);
      scope.autofocus(later);
      expect(first.hasPrimaryFocus, isFalse);
      await tester.pump();

      expect(first.hasPrimaryFocus, isTrue);
      expect(scope.focusedChild, same(first));
      expect(moved.hasPrimaryFocus, isFalse);
      expect(later.hasPrimaryFocus, isFalse);
    },
  );
}
