import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'a nearest scroll activity, not every offset change, requests hiding',
    (WidgetTester tester) async {
      final outer = ScrollController();
      final inner = ScrollController();
      addTearDown(outer.dispose);
      addTearDown(inner.dispose);
      final menu = MenuController();
      var requests = 0;
      VoidCallback? hideMenu;
      await tester.pumpWidget(
        WidgetsApp(
          color: const Color(0xFF000000),
          builder: (_, _) => Overlay(
            initialEntries: [
              OverlayEntry(
                builder: (_) => SingleChildScrollView(
                  controller: outer,
                  child: SizedBox(
                    height: 1200,
                    child: Align(
                      alignment: Alignment.topLeft,
                      child: SizedBox(
                        width: 300,
                        height: 250,
                        child: SingleChildScrollView(
                          controller: inner,
                          child: SizedBox(
                            height: 900,
                            child: RawMenuAnchor(
                              controller: menu,
                              child: const SizedBox(width: 30, height: 20),
                              onCloseRequested: (hide) {
                                requests++;
                                hideMenu = hide;
                              },
                              overlayBuilder: (_, _) => const SizedBox(),
                            ),
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
      );
      expect(inner.position.maxScrollExtent, 650);
      menu.open();
      await tester.pump();
      outer.jumpTo(30);
      inner.jumpTo(40);
      await tester.pump();
      expect(menu.isOpen, isTrue);
      expect(requests, 0);
      unawaited(
        outer.animateTo(
          60,
          duration: const Duration(milliseconds: 50),
          curve: Curves.linear,
        ),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 51));
      expect(menu.isOpen, isTrue);
      expect(requests, 0);
      unawaited(
        inner.animateTo(
          100,
          duration: const Duration(milliseconds: 100),
          curve: Curves.linear,
        ),
      );
      await tester.pump();
      expect(requests, 1);
      await tester.pump(const Duration(milliseconds: 50));
      expect(requests, 1);
      await tester.pump(const Duration(milliseconds: 50));
      await tester.pump(const Duration(milliseconds: 1));
      expect(requests, 2);
      hideMenu!();
      await tester.pump();
      expect(menu.isOpen, isFalse);
      inner.jumpTo(700);
      await tester.pumpAndSettle();
      expect(requests, 2);
    },
  );
}
