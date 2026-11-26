import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Escape dispatches once through the open focused menu tree', (
    WidgetTester tester,
  ) async {
    final root = MenuController();
    final child = MenuController();
    final inside = FocusNode();
    final outside = FocusNode();
    addTearDown(inside.dispose);
    addTearDown(outside.dispose);
    final requests = <String>[];
    var outerDismissals = 0;

    await tester.pumpWidget(
      WidgetsApp(
        color: const Color(0xFF000000),
        onGenerateRoute: (_) => PageRouteBuilder<void>(
          pageBuilder: (_, _, _) => Actions(
            actions: {
              DismissIntent: CallbackAction<DismissIntent>(
                onInvoke: (_) {
                  outerDismissals++;
                  return null;
                },
              ),
            },
            child: Column(
              children: [
                RawMenuAnchor(
                  controller: root,
                  child: Focus(
                    focusNode: inside,
                    child: const SizedBox(width: 100, height: 30),
                  ),
                  onCloseRequested: (_) => requests.add('root'),
                  overlayBuilder: (_, _) => RawMenuAnchor(
                    controller: child,
                    onCloseRequested: (_) => requests.add('nested'),
                    child: const SizedBox(width: 100, height: 30),
                    overlayBuilder: (_, _) => const SizedBox(),
                  ),
                ),
                Focus(
                  focusNode: outside,
                  child: const SizedBox(width: 100, height: 30),
                ),
              ],
            ),
          ),
        ),
      ),
    );

    inside.requestFocus();
    await tester.pump();
    expect(inside.hasPrimaryFocus, isTrue);
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    expect(outerDismissals, 1);
    expect(requests, isEmpty);

    root.open();
    await tester.pump();
    child.open();
    await tester.pump();
    requests.clear();
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    expect(requests, ['root', 'nested']);
    expect(outerDismissals, 1);
    expect(root.isOpen && child.isOpen, isTrue);

    outside.requestFocus();
    await tester.pump();
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    expect(outerDismissals, 2);
    expect(requests, ['root', 'nested']);
    expect(root.isOpen && child.isOpen, isTrue);
  });
}
