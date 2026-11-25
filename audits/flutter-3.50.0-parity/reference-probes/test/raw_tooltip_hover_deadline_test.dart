import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('hover schedules reveal and dismissal without pointer movement', (
    tester,
  ) async {
    const target = ValueKey('target');
    const tip = ValueKey('tip');
    final pointer = await tester.createGesture(kind: PointerDeviceKind.mouse);
    addTearDown(pointer.removePointer);
    await pointer.addPointer(location: Offset.zero);
    await tester.pumpWidget(
      WidgetsApp(
        color: const Color(0xFF000000),
        pageRouteBuilder: <T>(RouteSettings settings, WidgetBuilder builder) {
          return PageRouteBuilder<T>(
            settings: settings,
            pageBuilder: (context, _, _) => builder(context),
          );
        },
        home: const Center(
          child: RawTooltip(
            semanticsTooltip: "Hint",
            hoverDelay: Duration(milliseconds: 100),
            dismissDelay: Duration(milliseconds: 60),
            tooltipBuilder: buildTooltip,
            child: SizedBox(key: target, width: 100, height: 100),
          ),
        ),
      ),
    );
    await pointer.moveTo(tester.getCenter(find.byKey(target)));
    await tester.pump(const Duration(milliseconds: 99));
    expect(find.byKey(tip), findsNothing);
    await tester.pump(const Duration(milliseconds: 1));
    expect(find.byKey(tip), findsOneWidget);
    await tester.pump(const Duration(days: 1));
    expect(find.byKey(tip), findsOneWidget);
    await pointer.moveTo(Offset.zero);
    await tester.pump(const Duration(milliseconds: 59));
    expect(find.byKey(tip), findsOneWidget);
    await tester.pump(const Duration(milliseconds: 1));
    await tester.pumpAndSettle();
    expect(find.byKey(tip), findsNothing);
  });
}

Widget buildTooltip(BuildContext context, Animation<double> animation) =>
    const SizedBox(key: ValueKey('tip'), width: 40, height: 20);
