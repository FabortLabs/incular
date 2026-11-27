import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final explicit in [false, true]) {
    testWidgets(
      'WidgetsApp does not invent leaf autofocus: explicit=$explicit',
      (tester) async {
        final leaf = FocusNode();
        addTearDown(leaf.dispose);
        await tester.pumpWidget(
          WidgetsApp(
            color: const Color(0xFF123456),
            pageRouteBuilder: <T>(settings, builder) => PageRouteBuilder<T>(
              settings: settings,
              pageBuilder: (context, animation, secondaryAnimation) =>
                  Builder(builder: builder),
            ),
            home: Focus(
              focusNode: leaf,
              autofocus: explicit,
              child: const SizedBox(width: 25, height: 25),
            ),
          ),
        );
        await tester.pumpAndSettle();
        final outer = tester.widget<FocusScope>(
          find.byWidgetPredicate(
            (widget) =>
                widget is FocusScope && widget.debugLabel == 'Navigator Scope',
          ),
        );
        expect(outer.autofocus, isTrue);
        expect(leaf.hasPrimaryFocus, explicit);
        expect(leaf.enclosingScope, isNotNull);
      },
    );
  }
}
