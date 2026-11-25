import 'package:flutter/gestures.dart';
import 'package:flutter_test/flutter_test.dart';

class _Drag extends Drag {
  _Drag(this.start, this.events);

  final Offset start;
  final List<Object> events;

  @override
  void update(DragUpdateDetails details) {
    events.add((start, details.globalPosition, details.delta));
  }

  @override
  void end(DragEndDetails details) {
    events.add((start, 'end'));
  }

  @override
  void cancel() {
    events.add((start, 'cancel'));
  }
}

void main() {
  testWidgets('delayed drags use independent ordered deadlines and clients', (
    WidgetTester tester,
  ) async {
    final List<Object> events = <Object>[];
    final DelayedMultiDragGestureRecognizer drag =
        DelayedMultiDragGestureRecognizer();
    addTearDown(drag.dispose);
    drag.onStart = (Offset position) {
      events.add(position);
      return _Drag(position, events);
    };
    const Offset a = Offset(10, 10);
    const Offset b = Offset(100, 100);
    const Offset c = Offset(200, 200);
    final TestPointer first = TestPointer(90);
    final TestPointer second = TestPointer(10);
    final TestPointer third = TestPointer(2);
    final GestureBinding binding = GestureBinding.instance;

    void press(TestPointer pointer, Offset position) {
      final PointerDownEvent event = pointer.down(position);
      drag.addPointer(event);
      binding.pointerRouter.route(event);
      binding.gestureArena.close(pointer.pointer);
    }

    press(first, a);
    await tester.pump(const Duration(milliseconds: 20));
    press(second, b);
    press(third, c);
    await tester.pump(const Duration(milliseconds: 100));
    binding.pointerRouter.route(first.move(const Offset(13, 14)));
    binding.pointerRouter.route(second.move(const Offset(100, 107)));
    expect(events, isEmpty);

    await tester.pump(const Duration(milliseconds: 480));
    expect(events, <Object>[
      a,
      (a, a, const Offset(3, 4)),
      b,
      (b, b, const Offset(0, 7)),
      c,
      (c, c, Offset.zero),
    ]);

    binding.pointerRouter.route(second.move(const Offset(98, 111)));
    binding.pointerRouter.route(first.move(const Offset(18, 14)));
    binding.pointerRouter.route(first.cancel());
    binding.pointerRouter.route(second.up());
    binding.pointerRouter.route(third.up());
    expect(events.skip(6), <Object>[
      (b, const Offset(98, 111), const Offset(-2, 4)),
      (a, const Offset(18, 14), const Offset(5, 0)),
      (a, 'cancel'),
      (b, 'end'),
      (c, 'end'),
    ]);
  });
}
