import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class SemanticGestureHost extends SingleChildRenderObjectWidget {
  const SemanticGestureHost({
    required this.update,
    required this.tap,
    required this.allowed,
    required this.factor,
    super.key,
    super.child,
  });

  final GestureDragUpdateCallback update;
  final VoidCallback tap;
  final Set<SemanticsAction>? allowed;
  final double factor;

  @override
  RenderSemanticsGestureHandler createRenderObject(BuildContext context) =>
      RenderSemanticsGestureHandler(
        onHorizontalDragUpdate: update,
        onVerticalDragUpdate: update,
        onTap: tap,
        onLongPress: () {},
        scrollFactor: factor,
      )..validActions = allowed;

  @override
  void updateRenderObject(
    BuildContext context,
    RenderSemanticsGestureHandler renderObject,
  ) {
    renderObject
      ..onHorizontalDragUpdate = update
      ..onVerticalDragUpdate = update
      ..onTap = tap
      ..scrollFactor = factor
      ..validActions = allowed;
  }
}

void main() {
  testWidgets(
    'Semantic arrows synthesize exact drag updates and honor filters',
    (WidgetTester tester) async {
      const key = ValueKey<String>('gesture');
      final updates = <DragUpdateDetails>[];
      var taps = 0;
      Widget build(Set<SemanticsAction>? allowed, double factor) => Align(
        alignment: Alignment.topLeft,
        child: Transform.translate(
          offset: const Offset(20, 30),
          child: SemanticGestureHost(
            key: key,
            update: updates.add,
            tap: () => taps += 1,
            allowed: allowed,
            factor: factor,
            child: const SizedBox(width: 200, height: 100),
          ),
        ),
      );

      await tester.pumpWidget(build(null, 0.8));
      final render = tester.renderObject<RenderSemanticsGestureHandler>(
        find.byKey(key),
      );
      final initial = SemanticsConfiguration();
      render.describeSemanticsConfiguration(initial);
      const directions = [
        (SemanticsAction.scrollLeft, Offset(-160, 0), -160.0),
        (SemanticsAction.scrollRight, Offset(160, 0), 160.0),
        (SemanticsAction.scrollUp, Offset(0, -80), -80.0),
        (SemanticsAction.scrollDown, Offset(0, 80), 80.0),
      ];
      for (final (action, delta, primary) in directions) {
        initial.getActionHandler(action)!(null);
        final details = updates.last;
        expect(details.delta, delta);
        expect(details.primaryDelta, primary);
        expect(details.globalPosition, const Offset(120, 80));
        expect(details.localPosition, details.globalPosition);
        expect(details.sourceTimeStamp, isNull);
        expect(details.kind, isNull);
      }

      await tester.pumpWidget(
        build(const {
          SemanticsAction.tap,
          SemanticsAction.scrollLeft,
          SemanticsAction.scrollDown,
        }, 0.5),
      );
      expect(tester.renderObject(find.byKey(key)), same(render));
      final filtered = SemanticsConfiguration();
      render.describeSemanticsConfiguration(filtered);
      for (final action in [
        SemanticsAction.scrollRight,
        SemanticsAction.scrollUp,
        SemanticsAction.longPress,
      ]) {
        expect(filtered.getActionHandler(action), isNull);
      }
      filtered.getActionHandler(SemanticsAction.tap)!(null);
      filtered.getActionHandler(SemanticsAction.scrollLeft)!(null);
      filtered.getActionHandler(SemanticsAction.scrollDown)!(null);
      expect(taps, 1);
      expect(updates[4].delta, const Offset(-100, 0));
      expect(updates[5].delta, const Offset(0, 50));
    },
  );
}
