import 'dart:ui' show lerpDouble;

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

// Distinct forward and swapped paths. The interrupted flight must retrace
// the original cubic-in path, not create this tween with swapped endpoints.
class _AsymmetricRectTween extends RectTween {
  _AsymmetricRectTween({required super.begin, required super.end});

  @override
  Rect lerp(double t) {
    return Rect.fromLTWH(
      lerpDouble(begin!.left, end!.left, t * t * t)!,
      lerpDouble(begin!.top, end!.top, t)!,
      lerpDouble(begin!.width, end!.width, t)!,
      lerpDouble(begin!.height, end!.height, t)!,
    );
  }
}

void main() {
  testWidgets('an interrupted push retraces the same flying shuttle', (
    WidgetTester tester,
  ) async {
    final navigator = GlobalKey<NavigatorState>();
    var homeBuilds = 0;
    var detailBuilds = 0;
    const homeShuttle = ValueKey<String>('home shuttle');
    const detailShuttle = ValueKey<String>('detail shuttle');
    const duration = Duration(milliseconds: 300);
    const step = Duration(milliseconds: 30);

    Widget page(double x, bool details) {
      return Stack(
        children: <Widget>[
          Positioned(
            left: x,
            top: 20,
            child: Hero(
              tag: 'photo',
              curve: Curves.linear,
              createRectTween: (Rect? begin, Rect? end) =>
                  _AsymmetricRectTween(begin: begin, end: end),
              flightShuttleBuilder:
                  (
                    BuildContext context,
                    Animation<double> animation,
                    HeroFlightDirection direction,
                    BuildContext from,
                    BuildContext to,
                  ) {
                    if (details) {
                      detailBuilds++;
                    } else {
                      homeBuilds++;
                    }
                    return SizedBox(key: details ? detailShuttle : homeShuttle);
                  },
              child: const SizedBox(width: 40, height: 40),
            ),
          ),
        ],
      );
    }

    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Navigator(
          key: navigator,
          observers: <NavigatorObserver>[HeroController()],
          onGenerateRoute: (_) =>
              PageRouteBuilder<void>(pageBuilder: (_, _, _) => page(10, false)),
        ),
      ),
    );
    navigator.currentState!.push<void>(
      PageRouteBuilder<void>(
        transitionDuration: duration,
        reverseTransitionDuration: duration,
        pageBuilder: (_, _, _) => page(110, true),
      ),
    );
    await tester.pump();
    await tester.pump();
    final route = find.byKey(detailShuttle);
    final positions = <double>[tester.getTopLeft(route).dx];
    for (int i = 1; i <= 4; i++) {
      await tester.pump(step);
      positions.add(tester.getTopLeft(route).dx);
      expect(
        positions.last,
        closeTo(10 + 100 * (i / 10) * (i / 10) * (i / 10), 0.001),
      );
    }

    navigator.currentState!.pop<void>();
    await tester.pump();
    expect(tester.getTopLeft(route).dx, closeTo(positions.last, 0.001));
    for (int i = 3; i >= 0; i--) {
      await tester.pump(step);
      expect(tester.getTopLeft(route).dx, closeTo(positions[i], 0.001));
    }
    expect(homeBuilds, 0);
    expect(detailBuilds, 1);
    expect(find.byKey(homeShuttle), findsNothing);
    await tester.pump(const Duration(milliseconds: 1));
    expect(route, findsNothing);
  });
}
