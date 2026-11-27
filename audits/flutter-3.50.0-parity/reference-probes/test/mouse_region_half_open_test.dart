import 'dart:ui' show PointerDeviceKind;

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('MouseRegion includes top/left and excludes bottom/right', (tester) async {
    final events = <String>[];
    await tester.pumpWidget(Directionality(
      textDirection: TextDirection.ltr,
      child: Stack(children: [
        Positioned(
          left: 10,
          top: 20,
          width: 20,
          height: 10,
          child: MouseRegion(
            onEnter: (_) => events.add('enter'),
            onExit: (_) => events.add('exit'),
            child: const SizedBox.expand(),
          ),
        ),
      ]),
    ));
    final gesture = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await gesture.addPointer(location: const Offset(10, 20));
    expect(events, ['enter']);
    await gesture.moveTo(const Offset(29.75, 29.75));
    expect(events, ['enter']);
    await gesture.moveTo(const Offset(30, 25));
    expect(events, ['enter', 'exit']);
    await gesture.moveTo(const Offset(10, 20));
    expect(events, ['enter', 'exit', 'enter']);
    await gesture.moveTo(const Offset(15, 30));
    expect(events, ['enter', 'exit', 'enter', 'exit']);
    await gesture.removePointer();
  });
}
