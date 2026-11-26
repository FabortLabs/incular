import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('hiding the root detaches a nested dismissal action', (
    WidgetTester tester,
  ) async {
    final root = MenuController();
    final child = MenuController();
    final action = DismissMenuAction(controller: child);
    const intent = DismissIntent();
    expect(action.isEnabled(intent), isFalse);

    await tester.pumpWidget(
      WidgetsApp(
        color: const Color(0xFF000000),
        builder: (_, _) => Overlay(
          initialEntries: [
            OverlayEntry(
              builder: (_) => RawMenuAnchor(
                controller: root,
                child: const SizedBox(width: 30, height: 20),
                overlayBuilder: (_, _) => RawMenuAnchor(
                  controller: child,
                  child: const SizedBox(width: 30, height: 20),
                  overlayBuilder: (_, _) => const SizedBox(),
                ),
              ),
            ),
          ],
        ),
      ),
    );

    root.open();
    await tester.pump();
    expect(action.isEnabled(intent), isTrue, reason: 'mounted, but not open');
    child.open();
    await tester.pump();
    action.invoke(intent);
    await tester.pump();
    expect(root.isOpen || child.isOpen, isFalse);
    expect(action.isEnabled(intent), isFalse);
  });
}
