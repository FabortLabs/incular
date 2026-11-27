import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'unknown named routes are built once after the normal generator declines',
    (tester) async {
      final navigator = GlobalKey<NavigatorState>();
      final generated = <String?>[];
      final unknown = <String?>[];
      await tester.pumpWidget(
        WidgetsApp(
          navigatorKey: navigator,
          color: const Color(0xFF123456),
          home: const Text('home'),
          pageRouteBuilder: <T>(settings, builder) => PageRouteBuilder<T>(
            settings: settings,
            pageBuilder: (context, _, _) => builder(context),
          ),
          onGenerateRoute: (settings) {
            generated.add(settings.name);
            return null;
          },
          onUnknownRoute: (settings) {
            unknown.add(settings.name);
            return PageRouteBuilder<void>(
              settings: settings,
              pageBuilder: (_, _, _) => Text('fallback: ${settings.name}'),
            );
          },
        ),
      );
      expect(find.text('home'), findsOneWidget);
      expect(generated, isEmpty);
      expect(unknown, isEmpty);

      navigator.currentState!.pushNamed<void>('/unlisted');
      await tester.pumpAndSettle();
      expect(find.text('fallback: /unlisted'), findsOneWidget);
      expect(generated, ['/unlisted']);
      expect(unknown, ['/unlisted']);

      navigator.currentState!.pop();
      await tester.pumpAndSettle();
      expect(find.text('home'), findsOneWidget);
      expect(find.text('fallback: /unlisted'), findsNothing);
      expect(generated, ['/unlisted']);
      expect(unknown, ['/unlisted']);
    },
  );
}
