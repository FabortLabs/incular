import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('follower hit routing uses the current leader or fallback slot', (
    WidgetTester tester,
  ) async {
    final link = LayerLink();
    var calls = 0;
    Widget scene(Offset? leader, {bool showWhenUnlinked = false}) => Stack(
      textDirection: TextDirection.ltr,
      children: [
        if (leader != null)
          Positioned(
            left: leader.dx,
            top: leader.dy,
            child: CompositedTransformTarget(
              link: link,
              child: const SizedBox(width: 20, height: 20),
            ),
          ),
        Positioned.fill(
          key: const ValueKey('follower slot'),
          child: Align(
            alignment: Alignment.topLeft,
            child: CompositedTransformFollower(
              link: link,
              offset: const Offset(5, 10),
              showWhenUnlinked: showWhenUnlinked,
              child: GestureDetector(
                behavior: HitTestBehavior.opaque,
                onTap: () => calls++,
                child: const SizedBox(width: 20, height: 20),
              ),
            ),
          ),
        ),
      ],
    );

    const oldHit = Offset(65, 80);
    const newHit = Offset(145, 140);
    const layoutHit = Offset(10, 10);
    await tester.pumpWidget(scene(const Offset(50, 60)));
    await tester.tapAt(oldHit);
    expect(calls, 1);

    final held = await tester.startGesture(oldHit);
    await tester.pumpWidget(scene(null));
    await held.up();
    expect(calls, 2, reason: 'the already admitted contact retains its route');

    await tester.tapAt(oldHit);
    await tester.tapAt(layoutHit);
    expect(calls, 2, reason: 'a hidden orphan admits no new pointer');

    await tester.pumpWidget(scene(const Offset(130, 120)));
    await tester.tapAt(oldHit);
    await tester.tapAt(newHit);
    expect(calls, 3, reason: 'the follower only hits at the new leader');

    await tester.pumpWidget(scene(null, showWhenUnlinked: true));
    await tester.tapAt(newHit);
    await tester.tapAt(layoutHit);
    expect(calls, 4, reason: 'a visible orphan only hits at its layout slot');
  });
  testWidgets('moving a leader refreshes a stationary mouse over its follower', (
    WidgetTester tester,
  ) async {
    final link = LayerLink();
    final offset = ValueNotifier(Offset.zero);
    addTearDown(offset.dispose);
    final hover = <String>[];
    final focus = FocusNode();
    addTearDown(focus.dispose);
    await tester.pumpWidget(
      Stack(
        textDirection: TextDirection.ltr,
        children: [
          Positioned(
            left: 50,
            top: 60,
            child: ValueListenableBuilder<Offset>(
              valueListenable: offset,
              child: CompositedTransformTarget(
                link: link,
                child: const SizedBox(width: 20, height: 20),
              ),
              builder: (_, shift, child) =>
                  Transform.translate(offset: shift, child: child),
            ),
          ),
          Positioned.fill(
            child: Align(
              alignment: Alignment.topLeft,
              child: CompositedTransformFollower(
                link: link,
                offset: const Offset(5, 10),
                showWhenUnlinked: false,
                child: MouseRegion(
                  cursor: SystemMouseCursors.click,
                  onEnter: (_) => hover.add('enter'),
                  onExit: (_) => hover.add('exit'),
                  child: Focus(
                    focusNode: focus,
                    child: const SizedBox(width: 20, height: 20),
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await mouse.addPointer(location: const Offset(65, 80));
    await tester.pump();
    expect(hover, ['enter']);
    expect(focus.offset, const Offset(55, 70));

    offset.value = const Offset(80, 60);
    await tester.pump();
    expect(hover, ['enter', 'exit']);
    expect(focus.offset, const Offset(135, 130));

    offset.value = Offset.zero;
    await tester.pump();
    expect(hover, ['enter', 'exit', 'enter']);
    expect(focus.offset, const Offset(55, 70));
    await mouse.removePointer();
  });
}
