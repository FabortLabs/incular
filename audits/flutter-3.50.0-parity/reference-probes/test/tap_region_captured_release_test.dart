import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('TapRegion release uses the down hit path with the current pointer coordinates', (tester) async {
    final reports = <(String, Offset)>[];
    await tester.pumpWidget(Directionality(
      textDirection: TextDirection.ltr,
      child: Center(child: SizedBox(
        width: 200,
        height: 100,
        child: TapRegionSurface(
          child: ColoredBox(
            color: const Color(0xffeeeeee),
            child: Align(
              alignment: Alignment.centerLeft,
              child: TapRegion(
                onTapUpInside: (event) => reports.add(('inside', event.position)),
                onTapUpOutside: (event) => reports.add(('outside', event.position)),
                child: const SizedBox(width: 80, height: 80,
                  child: ColoredBox(color: Color(0xffffffff))),
              ),
            ),
          ),
        ),
      )),
    ));
    const inside = Offset(340, 300);
    const outsideRegion = Offset(460, 300);
    const outsideSurface = Offset(530, 300);

    Future<void> drag(Offset down, Offset up) async {
      final gesture = await tester.startGesture(down);
      await gesture.moveTo(up);
      await gesture.up();
    }

    await drag(inside, outsideRegion);
    expect(reports, [('inside', outsideRegion)]);
    reports.clear();

    await drag(outsideRegion, inside);
    expect(reports, [('outside', inside)]);
    reports.clear();

    await drag(inside, outsideSurface);
    expect(reports, [('inside', outsideSurface)]);
    reports.clear();

    await drag(outsideSurface, inside);
    expect(reports, isEmpty);

    final cancelled = await tester.startGesture(inside);
    await cancelled.moveTo(outsideRegion);
    await cancelled.cancel();
    expect(reports, isEmpty);
  });
}
