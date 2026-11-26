import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Authored Focus gates restore history and do not exclude a reparented leaf', (
    WidgetTester tester,
  ) async {
    final scope = FocusScopeNode(debugLabel: 'scope');
    final backup = FocusNode(debugLabel: 'backup');
    final bridge = FocusNode(debugLabel: 'bridge');
    final gate = FocusNode(debugLabel: 'gate');
    final leaf = FocusNode(debugLabel: 'leaf');
    for (final node in [scope, backup, bridge, gate, leaf]) {
      addTearDown(node.dispose);
    }

    Widget scene({required bool allowed, required bool explicit}) => FocusScope(
      node: scope,
      child: Column(
        children: [
          Focus.withExternalFocusNode(focusNode: backup, child: const SizedBox()),
          Focus.withExternalFocusNode(focusNode: bridge, child: const SizedBox()),
          Focus(
            focusNode: gate,
            descendantsAreFocusable: allowed,
            descendantsAreTraversable: allowed,
            child: Focus.withExternalFocusNode(
              focusNode: leaf,
              parentNode: explicit ? bridge : null,
              child: const SizedBox(),
            ),
          ),
        ],
      ),
    );

    await tester.pumpWidget(scene(allowed: true, explicit: false));
    backup.requestFocus();
    await tester.pump();
    leaf.requestFocus();
    await tester.pump();
    expect(leaf.hasPrimaryFocus, isTrue);
    expect(scope.focusedChild, leaf);

    await tester.pumpWidget(scene(allowed: false, explicit: false));
    expect(leaf.canRequestFocus, isFalse);
    expect(backup.hasPrimaryFocus, isTrue);
    expect(scope.focusedChild, backup);

    await tester.pumpWidget(scene(allowed: false, explicit: true));
    expect(leaf.canRequestFocus, isTrue);
    expect(leaf.skipTraversal, isFalse);
    leaf.requestFocus();
    await tester.pump();
    expect(leaf.hasPrimaryFocus, isTrue);
    expect(bridge.hasFocus, isTrue);
    expect(gate.hasFocus, isFalse);
  });
}
