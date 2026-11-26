import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('An opaque route can decline automatic focus', (tester) async {
    final navigator = GlobalKey<NavigatorState>();
    final focus = FocusNode();
    addTearDown(focus.dispose);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Navigator(
          key: navigator,
          onGenerateRoute: (settings) => PageRouteBuilder<void>(
            pageBuilder: (context, _, _) =>
                Focus(focusNode: focus, child: const Text('committed')),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    focus.requestFocus();
    await tester.pump();
    expect(focus.hasPrimaryFocus, isTrue);

    navigator.currentState!.push<void>(
      PageRouteBuilder<void>(
        requestFocus: false,
        pageBuilder: (context, _, _) => const Text('opt-out'),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('opt-out'), findsOneWidget);
    expect(focus.hasPrimaryFocus, isTrue);
    expect(
      FocusScope.of(tester.element(find.text('opt-out'))).hasFocus,
      isFalse,
    );
  });

  testWidgets('A focused dialog returns to the owner below an opt-out dialog', (
    tester,
  ) async {
    final navigator = GlobalKey<NavigatorState>();
    final focus = FocusNode();
    addTearDown(focus.dispose);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Navigator(
          key: navigator,
          onGenerateRoute: (_) => PageRouteBuilder<void>(
            pageBuilder: (context, _, _) =>
                Focus(focusNode: focus, child: const Text('committed')),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    focus.requestFocus();
    await tester.pump();
    expect(focus.hasPrimaryFocus, isTrue);

    void dialog(String label, bool requestFocus) {
      showGeneralDialog<void>(
        context: navigator.currentContext!,
        requestFocus: requestFocus,
        pageBuilder: (context, _, _) => Text(label),
      );
    }

    dialog('neutral', false);
    await tester.pumpAndSettle();
    expect(focus.hasPrimaryFocus, isTrue);
    dialog('focused', true);
    await tester.pumpAndSettle();
    expect(focus.hasFocus, isFalse);
    expect(
      FocusScope.of(tester.element(find.text('focused'))).hasPrimaryFocus,
      isTrue,
    );

    navigator.currentState!.pop();
    await tester.pumpAndSettle();
    expect(find.text('neutral'), findsOneWidget);
    expect(focus.hasPrimaryFocus, isTrue);
    expect(
      FocusScope.of(tester.element(find.text('neutral'))).hasFocus,
      isFalse,
    );
  });
}
