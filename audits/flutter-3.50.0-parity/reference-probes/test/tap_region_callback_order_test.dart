import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('outside callbacks precede inside callbacks in each phase', (
    t,
  ) async {
    const b = ValueKey<String>('region B');
    final trace = <String>[];
    await t.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: TapRegionSurface(
          child: Row(
            children: [
              TapRegion(
                behavior: HitTestBehavior.opaque,
                onTapOutside: (_) => trace.add('A.outsideDown'),
                onTapUpOutside: (_) => trace.add('A.outsideUp'),
                child: const SizedBox(width: 70, height: 32),
              ),
              TapRegion(
                key: b,
                behavior: HitTestBehavior.opaque,
                onTapInside: (_) => trace.add('B.insideDown'),
                onTapUpInside: (_) => trace.add('B.insideUp'),
                child: const SizedBox(width: 70, height: 32),
              ),
            ],
          ),
        ),
      ),
    );
    final gesture = await t.startGesture(
      t.getCenter(find.byKey(b)),
      kind: PointerDeviceKind.mouse,
    );
    await gesture.up();
    await gesture.removePointer();
    expect(trace, [
      'A.outsideDown',
      'B.insideDown',
      'A.outsideUp',
      'B.insideUp',
    ]);
  });
}
