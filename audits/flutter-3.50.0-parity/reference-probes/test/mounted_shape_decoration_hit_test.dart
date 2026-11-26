import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  Widget scene(
    Decoration decoration,
    Widget child,
    List<String> hits, [
    TextDirection direction = TextDirection.ltr,
  ]) => Directionality(
    textDirection: direction,
    child: Align(
      alignment: Alignment.topLeft,
      child: SizedBox(
        width: 120,
        height: 80,
        child: Stack(
          fit: StackFit.expand,
          children: <Widget>[
            GestureDetector(
              behavior: HitTestBehavior.opaque,
              onTap: () => hits.add('behind'),
              child: const SizedBox.expand(),
            ),
            GestureDetector(
              onTap: () => hits.add('shape'),
              child: DecoratedBox(decoration: decoration, child: child),
            ),
          ],
        ),
      ),
    ),
  );

  testWidgets('an unpainted corner passes through the decoration', (
    tester,
  ) async {
    final hits = <String>[];
    await tester.pumpWidget(
      scene(
        const ShapeDecoration(shape: CircleBorder(), color: Color(0xff123456)),
        const SizedBox.expand(),
        hits,
      ),
    );
    await tester.tapAt(const Offset(10, 10));
    await tester.tapAt(const Offset(60, 40));
    expect(hits, <String>['behind', 'shape']);
  });

  testWidgets('a real child can still hit outside the decoration', (
    tester,
  ) async {
    final hits = <String>[];
    await tester.pumpWidget(
      scene(
        const ShapeDecoration(shape: CircleBorder(), color: Color(0xff123456)),
        const ColoredBox(color: Color(0xffffffff)),
        hits,
      ),
    );
    await tester.tapAt(const Offset(10, 10));
    expect(hits, <String>['shape']);
  });

  testWidgets('changing direction moves the rounded hit corner', (
    tester,
  ) async {
    final hits = <String>[];
    const decoration = ShapeDecoration(
      color: Color(0xff123456),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadiusDirectional.only(
          topStart: Radius.circular(30),
        ),
      ),
    );
    await tester.pumpWidget(scene(decoration, const SizedBox.expand(), hits));
    await tester.tapAt(const Offset(4, 4));
    await tester.pumpWidget(
      scene(decoration, const SizedBox.expand(), hits, TextDirection.rtl),
    );
    await tester.tapAt(const Offset(4, 4));
    expect(hits, <String>['behind', 'shape']);
  });

  testWidgets('mounted decoration honors a shape-specific hit override', (
    tester,
  ) async {
    final hits = <String>[];
    await tester.pumpWidget(
      scene(
        const ShapeDecoration(shape: _NoHitBorder(), color: Color(0xff123456)),
        const SizedBox.expand(),
        hits,
      ),
    );
    await tester.tapAt(const Offset(60, 40));
    expect(hits, <String>['behind']);
  });
}

class _NoHitBorder extends ShapeBorder {
  const _NoHitBorder();

  @override
  EdgeInsetsGeometry get dimensions => EdgeInsets.zero;

  @override
  ShapeBorder scale(double t) => this;

  @override
  Path getInnerPath(Rect rect, {TextDirection? textDirection}) =>
      Path()..addRect(rect);

  @override
  Path getOuterPath(Rect rect, {TextDirection? textDirection}) =>
      Path()..addRect(rect);

  @override
  bool hitTest(Rect rect, Offset position, {TextDirection? textDirection}) =>
      false;

  @override
  void paint(Canvas canvas, Rect rect, {TextDirection? textDirection}) {}
}
