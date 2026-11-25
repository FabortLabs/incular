import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'scope history is preserved by direct scope focus and follows a reparent',
    (tester) async {
      final left = FocusScopeNode();
      final right = FocusScopeNode();
      final child = FocusNode();
      addTearDown(left.dispose);
      addTearDown(right.dispose);
      addTearDown(child.dispose);
      final key = GlobalKey();
      final routes = <int>[];
      Widget scene({required bool onLeft}) {
        Widget slot({
          required bool here,
          required FocusScopeNode node,
          required int route,
        }) {
          return FocusScope.withExternalFocusNode(
            focusScopeNode: node,
            child: Focus(
              onKeyEvent: (_, _) {
                routes.add(route);
                return KeyEventResult.ignored;
              },
              child: here
                  ? Focus(key: key, focusNode: child, child: const Text('a'))
                  : const SizedBox.shrink(),
            ),
          );
        }

        return Directionality(
          textDirection: TextDirection.ltr,
          child: Column(
            children: [
              slot(here: onLeft, node: left, route: 0),
              slot(here: !onLeft, node: right, route: 1),
            ],
          ),
        );
      }

      await tester.pumpWidget(scene(onLeft: true));
      child.requestFocus();
      await tester.pump();
      final element = key.currentContext;
      expect(left.focusedChild, child);
      expect(right.focusedChild, isNull);

      left.requestScopeFocus();
      await tester.pump();
      expect(left.hasPrimaryFocus, isTrue);
      expect(left.focusedChild, child);
      child.requestFocus();
      await tester.pump();

      await tester.pumpWidget(scene(onLeft: false));
      expect(key.currentContext, same(element));
      expect(child.hasPrimaryFocus, isTrue);
      expect(left.focusedChild, isNull);
      expect(right.focusedChild, child);
      expect(await simulateKeyDownEvent(LogicalKeyboardKey.digit1), isFalse);
      expect(routes, [1]);
      await simulateKeyUpEvent(LogicalKeyboardKey.digit1);

      right.requestScopeFocus();
      await tester.pump();
      expect(right.hasPrimaryFocus, isTrue);
      await tester.pumpWidget(scene(onLeft: true));
      expect(child.hasFocus, isFalse);
      expect(left.focusedChild, isNull);
      expect(right.focusedChild, isNull);
    },
  );

  testWidgets('outer scope remembers the immediate nested scope', (
    tester,
  ) async {
    final outer = FocusScopeNode();
    final nested = FocusScopeNode();
    final leaf = FocusNode();
    addTearDown(outer.dispose);
    addTearDown(nested.dispose);
    addTearDown(leaf.dispose);
    await tester.pumpWidget(
      FocusScope.withExternalFocusNode(
        focusScopeNode: outer,
        child: FocusScope.withExternalFocusNode(
          focusScopeNode: nested,
          child: Focus(focusNode: leaf, child: const SizedBox.shrink()),
        ),
      ),
    );
    leaf.requestFocus();
    await tester.pump();
    expect(leaf.hasPrimaryFocus, isTrue);
    expect(outer.focusedChild, nested);
    expect(nested.focusedChild, leaf);

    nested.requestScopeFocus();
    await tester.pump();
    expect(outer.focusedChild, nested);
    expect(nested.hasPrimaryFocus, isTrue);
    expect(nested.focusedChild, leaf);

    outer.requestFocus();
    await tester.pump();
    expect(leaf.hasPrimaryFocus, isTrue);
    leaf.canRequestFocus = false;
    outer.requestFocus();
    await tester.pump();
    expect(nested.hasPrimaryFocus, isTrue);
    expect(leaf.hasFocus, isFalse);
  });
}
