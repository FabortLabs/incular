import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _EagerPan extends PanGestureRecognizer {
  @override
  bool hasSufficientGlobalDistanceToAccept(
    PointerDeviceKind kind,
    double? touchSlop,
  ) {
    return globalDistanceMoved.abs() > computeHitSlop(kind, gestureSettings);
  }
}

void main() {
  for (final bool eager in [true, false]) {
    testWidgets(
      'raw pan and scroll share the same pointer arena (eager: $eager)',
      (tester) async {
        final controller = ScrollController(initialScrollOffset: 30);
        addTearDown(controller.dispose);
        const key = ValueKey<String>('pan target');
        final trace = <String>[];
        void configure(PanGestureRecognizer recognizer) {
          recognizer.onStart = (_) => trace.add('pan start');
          recognizer.onEnd = (_) => trace.add('pan end');
        }

        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: CustomScrollView(
              controller: controller,
              slivers: <Widget>[
                SliverToBoxAdapter(
                  child: RawGestureDetector(
                    behavior: HitTestBehavior.translucent,
                    gestures: <Type, GestureRecognizerFactory>{
                      if (eager)
                        _EagerPan:
                            GestureRecognizerFactoryWithHandlers<_EagerPan>(
                              _EagerPan.new,
                              configure,
                            )
                      else
                        PanGestureRecognizer:
                            GestureRecognizerFactoryWithHandlers<
                              PanGestureRecognizer
                            >(PanGestureRecognizer.new, configure),
                    },
                    child: const SizedBox(key: key, width: 100, height: 100),
                  ),
                ),
                const SliverToBoxAdapter(child: SizedBox(height: 1000)),
              ],
            ),
          ),
        );
        final gesture = await tester.startGesture(
          tester.getCenter(find.byKey(key)),
        );
        await tester.pump();
        expect(trace, isEmpty);
        await gesture.moveBy(const Offset(30, kTouchSlop + 1));
        await tester.pump();
        expect(trace, eager ? ['pan start'] : isEmpty);
        await gesture.moveBy(const Offset(0, 10));
        await tester.pump();
        expect(controller.offset, eager ? 30 : 20);
        await gesture.up();
        expect(trace, eager ? ['pan start', 'pan end'] : isEmpty);
      },
    );
  }
}
