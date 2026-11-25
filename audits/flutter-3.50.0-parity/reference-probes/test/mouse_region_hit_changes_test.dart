import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

typedef Observation = (String, String);

Widget overlap(List<Observation> log, bool opaque) => Directionality(
  textDirection: TextDirection.ltr,
  child: Align(
    alignment: Alignment.topLeft,
    child: Stack(
      children: [
        SizedBox.square(
          dimension: 50,
          child: MouseRegion(
            onEnter: (_) => log.add(('bottom', 'enter')),
            onExit: (_) => log.add(('bottom', 'exit')),
            onHover: (_) => log.add(('bottom', 'hover')),
          ),
        ),
        SizedBox.square(
          dimension: 50,
          child: MouseRegion(
            opaque: opaque,
            onEnter: (_) => log.add(('top', 'enter')),
            onExit: (_) => log.add(('top', 'exit')),
            onHover: (_) => log.add(('top', 'hover')),
          ),
        ),
      ],
    ),
  ),
);

void main() {
  testWidgets('opacity changes only affect the covered region', (tester) async {
    final log = <Observation>[];
    await tester.pumpWidget(overlap(log, true));
    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await mouse.addPointer(location: const Offset(10, 10));
    expect(log, [('top', 'enter')]);
    log.clear();
    await tester.pumpWidget(overlap(log, false));
    expect(log, [('bottom', 'enter')]);
    log.clear();
    await tester.pumpWidget(overlap(log, true));
    expect(log, [('bottom', 'exit')]);
    await mouse.removePointer();
  });

  testWidgets('touch contact motion is not hover', (tester) async {
    final log = <String>[];
    await tester.pumpWidget(
      MouseRegion(
        onEnter: (_) => log.add('enter'),
        onHover: (_) => log.add('hover'),
        onExit: (_) => log.add('exit'),
      ),
    );
    final touch = await tester.startGesture(const Offset(30, 30));
    await touch.moveTo(const Offset(31, 30));
    expect(log, isEmpty);
    await touch.up();
  });
}
