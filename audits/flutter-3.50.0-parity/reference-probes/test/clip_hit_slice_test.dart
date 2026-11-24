import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class Triangle extends CustomClipper<Path> {
  const Triangle();
  @override
  Path getClip(Size s) => Path()
    ..moveTo(0, 0)
    ..lineTo(s.width, 0)
    ..lineTo(0, s.height)
    ..close();
  @override
  bool shouldReclip(Triangle old) => false;
}

void main() {
  testWidgets('custom clip rejects a hit in the box but outside the path', (
    t,
  ) async {
    const key = ValueKey('box');
    var taps = 0;
    await t.pumpWidget(
      Center(
        child: ClipPath(
          clipper: const Triangle(),
          child: GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTap: () => taps++,
            child: const SizedBox(key: key, width: 100, height: 100),
          ),
        ),
      ),
    );
    final p = t.getTopLeft(find.byKey(key));
    await t.tapAt(p + const Offset(20, 20));
    expect(taps, 1);
    await t.tapAt(p + const Offset(80, 80));
    expect(taps, 1);
  });
}
