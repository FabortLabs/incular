import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Directional alignment mirrors while the size keeps animating', (WidgetTester tester) async {
    final Key child = UniqueKey();
    var ended = 0;
    Widget build(TextDirection direction, double side) => Directionality(
      textDirection: direction,
      child: Center(
        child: AnimatedSize(
          duration: const Duration(milliseconds: 200),
          clipBehavior: Clip.none,
          alignment: AlignmentDirectional.centerStart,
          onEnd: () => ended++,
          child: SizedBox(key: child, width: side, height: side),
        ),
      ),
    );

    final Finder outer = find.byType(AnimatedSize);
    final Finder inner = find.byKey(child);
    Offset offset() => tester.getTopLeft(inner) - tester.getTopLeft(outer);
    await tester.pumpWidget(build(TextDirection.ltr, 100));
    expect(tester.getSize(outer), const Size.square(100));
    await tester.pumpWidget(build(TextDirection.ltr, 200));
    await tester.pump(const Duration(milliseconds: 100));
    expect(tester.getSize(outer), const Size.square(150));
    expect(offset(), const Offset(0, -25));

    await tester.pumpWidget(build(TextDirection.rtl, 200));
    expect(tester.getSize(outer), const Size.square(150));
    expect(offset(), const Offset(-50, -25));
    expect(ended, 0);
    await tester.pump(const Duration(milliseconds: 100));
    expect(tester.getSize(outer), const Size.square(200));
    expect(offset(), Offset.zero);
    expect(ended, 0);
    await tester.pump(const Duration(milliseconds: 1));
    expect(ended, 1);
  });
}
