import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

Widget app(Widget child) => WidgetsApp(
  color: const Color(0xFF000000),
  onGenerateRoute: (_) =>
      PageRouteBuilder<void>(pageBuilder: (_, _, _) => Center(child: child)),
);

void main() {
  testWidgets('anchor rectangle uses the composed world transform', (
    WidgetTester tester,
  ) async {
    final menu = MenuController();
    final key = GlobalKey();
    RawMenuOverlayInfo? last;
    await tester.pumpWidget(
      app(
        Transform(
          transform: Matrix4.translationValues(-50, 50, 0)
            ..scaleByDouble(1.2, 1.2, 1.2, 1),
          child: RawMenuAnchor(
            controller: menu,
            child: SizedBox(key: key, width: 100, height: 20),
            overlayBuilder: (_, info) {
              last = info;
              return const SizedBox();
            },
          ),
        ),
      ),
    );
    menu.open();
    await tester.pump();
    expect(last!.anchorRect, tester.getRect(find.byKey(key)));
    expect(last!.anchorRect, const Rect.fromLTWH(300, 340, 120, 24));
    expect(last!.overlaySize, const Size(800, 600));
  });

  testWidgets('removing a 100 pixel leading child updates the live rectangle', (
    WidgetTester tester,
  ) async {
    final menu = MenuController();
    final key = GlobalKey();
    RawMenuOverlayInfo? last;
    late StateSetter update;
    var extra = true;
    final anchor = RawMenuAnchor(
      controller: menu,
      child: SizedBox(key: key, width: 100, height: 20),
      overlayBuilder: (_, info) {
        last = info;
        return const SizedBox();
      },
    );
    await tester.pumpWidget(
      app(
        StatefulBuilder(
          builder: (_, set) {
            update = set;
            return Column(
              children: [
                if (extra) const SizedBox(width: 100, height: 100),
                anchor,
              ],
            );
          },
        ),
      ),
    );
    menu.open();
    await tester.pump();
    final before = last!.anchorRect;
    expect(before, tester.getRect(find.byKey(key)));
    update(() => extra = false);
    await tester.pumpAndSettle();
    final after = last!.anchorRect;
    expect(after, tester.getRect(find.byKey(key)));
    expect(before.top - after.top, 100);
    expect(menu.isOpen, isTrue);
  });
}
