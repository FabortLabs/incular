import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _InsetPath extends CustomClipper<Path> {
  const _InsetPath();

  @override
  Path getClip(Size size) =>
      Path()..addRect(const Rect.fromLTRB(20, 10, 80, 30));

  @override
  bool shouldReclip(_InsetPath oldClipper) => false;
}

void main() {
  testWidgets('physical clip mode does not change hit geometry', (
    tester,
  ) async {
    const size = SizedBox(width: 100, height: 40);
    for (final clip in [Clip.none, Clip.antiAlias]) {
      for (final custom in [false, true]) {
        var hits = 0;
        final child = GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTap: () => hits++,
          child: const SizedBox.expand(),
        );
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: Align(
              alignment: Alignment.topLeft,
              child: SizedBox(
                width: size.width,
                height: size.height,
                child: custom
                    ? PhysicalShape(
                        clipper: const _InsetPath(),
                        color: const Color(0xFFFFFFFF),
                        clipBehavior: clip,
                        child: child,
                      )
                    : PhysicalModel(
                        shape: BoxShape.circle,
                        color: const Color(0xFFFFFFFF),
                        clipBehavior: clip,
                        child: child,
                      ),
              ),
            ),
          ),
        );
        await tester.tapAt(const Offset(2, 2));
        expect(hits, custom ? 0 : 1);
        await tester.tapAt(const Offset(50, 20));
        expect(hits, custom ? 1 : 2);
      }
    }
  });
}
