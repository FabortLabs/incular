import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class RecordingPaintingContext implements PaintingContext {
  final List<Invocation> invocations = [];

  @override
  dynamic noSuchMethod(Invocation invocation) {
    invocations.add(invocation);
    return null;
  }
}

void main() {
  for (final indexed in [false, true]) {
    testWidgets(
      '${indexed ? 'IndexedStack' : 'Stack'} paints the chosen clip coverage',
      (WidgetTester tester) async {
        RenderStack? initial;
        for (final mode in [
          Clip.hardEdge,
          Clip.antiAlias,
          Clip.antiAliasWithSaveLayer,
          Clip.none,
          Clip.hardEdge,
        ]) {
          const children = [
            SizedBox(width: 100, height: 100),
            Positioned(
              left: 0,
              top: 0,
              child: ColoredBox(
                color: Color(0xFFFFFFFF),
                child: SizedBox(width: 180, height: 180),
              ),
            ),
          ];
          final layout = indexed
              ? IndexedStack(index: 1, clipBehavior: mode, children: children)
              : Stack(clipBehavior: mode, children: children);
          await tester.pumpWidget(
            Directionality(
              textDirection: TextDirection.ltr,
              child: Center(child: layout),
            ),
          );
          final render = tester.renderObject<RenderStack>(
            find.byType(layout.runtimeType),
          );
          initial ??= render;
          expect(render, same(initial));
          expect(render.clipBehavior, mode);
          expect(render.size, const Size(100, 100));
          final context = RecordingPaintingContext();
          render.paint(context, Offset.zero);
          final clips = context.invocations
              .where((call) => call.memberName == #pushClipRect)
              .toList();
          if (mode == Clip.none) {
            expect(clips, isEmpty);
          } else {
            expect(clips.single.namedArguments[#clipBehavior], mode);
            expect(
              clips.single.positionalArguments[2],
              const Rect.fromLTWH(0, 0, 100, 100),
            );
          }
        }
      },
    );
  }
}
