import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('a parent requests closes, but hiding it closes children first', (
    WidgetTester tester,
  ) async {
    final parent = MenuController();
    final child = MenuController();
    final log = <String>[];
    VoidCallback? hideParent;
    await tester.pumpWidget(
      WidgetsApp(
        color: const Color(0xFF000000),
        builder: (_, _) => Overlay(
          initialEntries: [
            OverlayEntry(
              builder: (_) => RawMenuAnchor(
                controller: parent,
                onCloseRequested: (hide) {
                  log.add('request parent');
                  hideParent = hide;
                },
                onClose: () => log.add('close parent'),
                child: const SizedBox(width: 30, height: 20),
                overlayBuilder: (_, _) => RawMenuAnchor(
                  controller: child,
                  onCloseRequested: (_) => log.add('request child'),
                  onClose: () => log.add('close child'),
                  child: const SizedBox(width: 30, height: 20),
                  overlayBuilder: (_, _) => const SizedBox(),
                ),
              ),
            ),
          ],
        ),
      ),
    );
    parent.open();
    await tester.pump();
    child.open();
    await tester.pump();
    log.clear();

    parent.close();
    expect(parent.isOpen && child.isOpen, isTrue);
    expect(log, ['request parent', 'request child']);
    hideParent!();
    await tester.pump();
    expect(parent.isOpen || child.isOpen, isFalse);
    expect(log, [
      'request parent',
      'request child',
      'close child',
      'close parent',
    ]);
  });
  testWidgets('an attached hidden anchor still receives a close request', (
    WidgetTester tester,
  ) async {
    final menu = MenuController();
    var requests = 0;
    var closes = 0;
    await tester.pumpWidget(
      WidgetsApp(
        color: const Color(0xFF000000),
        builder: (_, _) => Overlay(
          initialEntries: [
            OverlayEntry(
              builder: (_) => RawMenuAnchor(
                controller: menu,
                overlayBuilder: (_, _) => const SizedBox(),
                onCloseRequested: (hide) {
                  requests++;
                  hide();
                },
                onClose: () => closes++,
              ),
            ),
          ],
        ),
      ),
    );
    menu.open();
    await tester.pump();
    menu.close();
    await tester.pump();
    expect(menu.isOpen, isFalse);
    expect((requests, closes), (1, 1));
    menu.close();
    await tester.pump();
    expect(menu.isOpen, isFalse);
    expect((requests, closes), (2, 1));
  });
}
