import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class Recorder extends NavigatorObserver {
  final events = <String>[];

  @override
  void didPush(Route<dynamic> route, Route<dynamic>? previousRoute) {
    events.add('push:${route.settings.name}:${previousRoute?.settings.name}');
  }

  @override
  void didRemove(Route<dynamic> route, Route<dynamic>? previousRoute) {
    events.add('remove:${route.settings.name}:${previousRoute?.settings.name}');
  }

  @override
  void didPop(Route<dynamic> route, Route<dynamic>? previousRoute) {
    events.add('pop:${route.settings.name}:${previousRoute?.settings.name}');
  }

  @override
  void didChangeTop(Route<dynamic> topRoute, Route<dynamic>? previousTopRoute) {
    events.add(
      'top:${topRoute.settings.name}:${previousTopRoute?.settings.name}',
    );
  }
}

PageRoute<void> page(RouteSettings settings) => PageRouteBuilder<void>(
  settings: settings,
  pageBuilder: (_, _, _) => Text(settings.name!),
  transitionDuration: Duration.zero,
  reverseTransitionDuration: Duration.zero,
);

void main() {
  testWidgets('bulk removal reports the surviving predecessor in order', (
    WidgetTester tester,
  ) async {
    final key = GlobalKey<NavigatorState>();
    final recorder = Recorder();
    await tester.pumpWidget(
      WidgetsApp(
        color: const Color(0xFF000000),
        navigatorKey: key,
        navigatorObservers: [recorder],
        onGenerateRoute: page,
      ),
    );
    final navigator = key.currentState!;
    navigator.push(page(const RouteSettings(name: '/A')));
    await tester.pumpAndSettle();
    navigator.push(page(const RouteSettings(name: '/B')));
    await tester.pumpAndSettle();
    recorder.events.clear();
    navigator.pushAndRemoveUntil(
      page(const RouteSettings(name: '/C')),
      ModalRoute.withName('/'),
    );
    await tester.pumpAndSettle();

    expect(recorder.events, [
      'push:/C:/B',
      'remove:/B:/',
      'remove:/A:/',
      'top:/C:/B',
    ]);
    expect(find.text('/C'), findsOneWidget);
    expect(find.text('/A', skipOffstage: false), findsNothing);
    expect(find.text('/B', skipOffstage: false), findsNothing);
  });
}
