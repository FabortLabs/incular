import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class RecordingClipContext extends PaintingContext {
  RecordingClipContext(this.container) : super(container, Rect.zero);

  final ContainerLayer container;
  final List<(Rect, Clip)> clips = [];

  @override
  void paintChild(RenderObject child, Offset offset) {}

  @override
  ClipRectLayer? pushClipRect(
    bool needsCompositing,
    Offset offset,
    Rect clipRect,
    PaintingContextCallback painter, {
    Clip clipBehavior = Clip.hardEdge,
    ClipRectLayer? oldLayer,
  }) {
    clips.add((clipRect.shift(offset), clipBehavior));
    return null;
  }

  void dispose() {
    stopRecordingIfNeeded();
    container.dispose();
  }
}

void main() {
  testWidgets('A retained flex removes its clip when its content fits', (
    WidgetTester tester,
  ) async {
    const key = ValueKey<String>('flex');
    Widget build(double height) => Directionality(
      textDirection: TextDirection.ltr,
      child: Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 100,
          height: height,
          child: Flex(
            direction: Axis.vertical,
            key: key,
            clipBehavior: Clip.antiAliasWithSaveLayer,
            children: const [SizedBox(width: 200, height: 200)],
          ),
        ),
      ),
    );

    List<(Rect, Clip)> inspect(RenderFlex flex) {
      final context = RecordingClipContext(ContainerLayer());
      flex.paint(context, Offset.zero);
      tester.takeException(); // The debug overflow reporter is a separate surface.
      final result = context.clips.toList();
      context.dispose();
      return result;
    }

    await tester.pumpWidget(build(100));
    expect(tester.takeException(), isA<FlutterError>());
    final render = tester.renderObject<RenderFlex>(find.byKey(key));
    const clipped = [
      (Rect.fromLTWH(0, 0, 100, 100), Clip.antiAliasWithSaveLayer),
    ];
    expect(inspect(render), clipped);

    await tester.pumpWidget(build(250));
    expect(tester.takeException(), isNull);
    expect(tester.renderObject(find.byKey(key)), same(render));
    expect(inspect(render), isEmpty);

    await tester.pumpWidget(build(100));
    tester.takeException();
    expect(tester.renderObject(find.byKey(key)), same(render));
    expect(inspect(render), clipped);
  });

  testWidgets('Fractional flex shares do not trigger the overflow clip', (
    WidgetTester tester,
  ) async {
    const key = ValueKey<String>('fractional-flex');
    await tester.pumpWidget(
      const Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: 500,
            child: Flex(
              direction: Axis.horizontal,
              key: key,
              clipBehavior: Clip.hardEdge,
              children: [
                Expanded(child: SizedBox(height: 20)),
                Expanded(child: SizedBox(height: 20)),
                Expanded(child: SizedBox(height: 20)),
              ],
            ),
          ),
        ),
      ),
    );
    expect(tester.takeException(), isNull);
    final flex = tester.renderObject<RenderFlex>(find.byKey(key));
    expect(flex.size, const Size(500, 20));
    var child = flex.firstChild;
    for (int i = 0; i < 3; i++) {
      expect(child!.size.width, closeTo(500 / 3, 1e-10));
      final data = child.parentData! as FlexParentData;
      expect(data.offset.dx, closeTo(i * 500 / 3, 1e-10));
      child = flex.childAfter(child);
    }
    final context = RecordingClipContext(ContainerLayer());
    flex.paint(context, Offset.zero);
    expect(context.clips, isEmpty);
    context.dispose();
  });
}
