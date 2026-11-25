import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

const _bar = ValueKey('bar');
const _first = ValueKey('first');
const _children = [
  SizedBox(key: _first, width: 48, height: 48),
  SizedBox(width: 64, height: 64),
  SizedBox(width: 32, height: 32),
];

void main() {
  Widget frame(Widget child) => Directionality(
    textDirection: TextDirection.ltr,
    child: Align(alignment: Alignment.topLeft, child: child),
  );

  testWidgets('exact fit becomes an upward overflow on the same child', (
    tester,
  ) async {
    Widget build(double width) => frame(
      SizedBox(
        width: width,
        child: const OverflowBar(
          key: _bar,
          spacing: 3,
          textDirection: TextDirection.rtl,
          overflowDirection: VerticalDirection.up,
          children: _children,
        ),
      ),
    );

    await tester.pumpWidget(build(150));
    final first = find.byKey(_first);
    final original = tester.element(first);
    expect(tester.getTopLeft(first), const Offset(102, 8));
    await tester.pumpWidget(build(149));
    expect(tester.element(first), same(original));
    expect(tester.getTopLeft(first), const Offset(101, 96));
  });

  testWidgets('intrinsic width does not expand with explicit row alignment', (
    tester,
  ) async {
    await tester.pumpWidget(
      frame(
        const IntrinsicWidth(
          child: OverflowBar(
            key: _bar,
            spacing: 3,
            alignment: MainAxisAlignment.end,
            children: _children,
          ),
        ),
      ),
    );
    expect(tester.getSize(find.byKey(_bar)), const Size(150, 64));
    expect(tester.getTopLeft(find.byKey(_first)), const Offset(0, 8));
  });

  testWidgets('single space-between child stays at logical start', (
    tester,
  ) async {
    await tester.pumpWidget(
      frame(
        ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 100),
          child: const OverflowBar(
            key: _bar,
            alignment: MainAxisAlignment.spaceBetween,
            textDirection: TextDirection.rtl,
            children: [SizedBox(key: _first, width: 48, height: 30)],
          ),
        ),
      ),
    );
    expect(tester.getSize(find.byKey(_bar)), const Size(100, 30));
    expect(tester.getTopLeft(find.byKey(_first)), const Offset(52, 0));
  });
}
