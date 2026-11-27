import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final host in ['/', '/inbox/thread']) {
    testWidgets('startup chooses a non-root host route: $host', (tester) async {
      tester.platformDispatcher.defaultRouteNameTestValue = host;
      addTearDown(tester.platformDispatcher.clearDefaultRouteNameTestValue);
      final calls = <String>[];
      final expected = host == '/' ? '/welcome' : host;
      await tester.pumpWidget(
        WidgetsApp(
          color: const Color(0xFF123456),
          initialRoute: '/welcome',
          onGenerateRoute: (_) => null,
          onGenerateInitialRoutes: (name) {
            calls.add(name);
            return [
              PageRouteBuilder<void>(
                settings: RouteSettings(name: name),
                pageBuilder: (context, _, _) => Text('opened: $name'),
              ),
            ];
          },
        ),
      );
      expect(calls, [expected]);
      expect(find.text('opened: $expected'), findsOneWidget);
    });
  }

  for (final exists in [true, false]) {
    testWidgets(
      'startup generates root and prefix before the leaf: exists=$exists',
      (tester) async {
        final calls = <String?>[];
        await tester.pumpWidget(
          WidgetsApp(
            color: const Color(0xFF123456),
            initialRoute: '/a/b',
            onGenerateRoute: (settings) {
              calls.add(settings.name);
              if (!exists && settings.name == '/a/b') {
                return null;
              }
              return PageRouteBuilder<void>(
                settings: settings,
                pageBuilder: (context, _, _) =>
                    Text('generated: ${settings.name}'),
              );
            },
          ),
        );
        if (exists) {
          expect(tester.takeException(), isNull);
          expect(calls, ['/', '/a', '/a/b']);
          expect(find.text('generated: /a/b'), findsOneWidget);
        } else {
          expect(
            tester.takeException().toString(),
            contains('Could not navigate to initial route.'),
          );
          expect(calls, ['/', '/a', '/a/b', '/']);
          expect(find.text('generated: /'), findsOneWidget);
          expect(find.text('generated: /a'), findsNothing);
        }
      },
    );
  }
}
