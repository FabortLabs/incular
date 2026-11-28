import 'package:flutter/rendering.dart' show LayerLink;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('zero-alpha leader unlinks its follower until it paints again', (
    WidgetTester tester,
  ) async {
    final link = LayerLink();
    final key = GlobalKey();
    var taps = 0;
    for (final (opacity, visible) in [
      (1.0, true),
      (0.0, false),
      (0.001, false),
      (0.5, true),
    ]) {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Stack(
            children: [
              Positioned(
                left: 30,
                top: 40,
                child: Opacity(
                  opacity: opacity,
                  child: CompositedTransformTarget(
                    link: link,
                    child: const SizedBox.square(dimension: 24),
                  ),
                ),
              ),
              Positioned(
                left: 100,
                top: 100,
                child: CompositedTransformFollower(
                  link: link,
                  showWhenUnlinked: false,
                  child: GestureDetector(
                    key: key,
                    behavior: HitTestBehavior.opaque,
                    onTap: () => taps++,
                    child: const SizedBox.square(dimension: 20),
                  ),
                ),
              ),
            ],
          ),
        ),
      );
      expect(link.leader != null, visible, reason: 'opacity $opacity');
      final before = taps;
      await tester.tapAt(const Offset(35, 45));
      expect(taps - before, visible ? 1 : 0, reason: 'opacity $opacity');
      expect(find.byKey(key), findsOneWidget);
    }
  });
}
