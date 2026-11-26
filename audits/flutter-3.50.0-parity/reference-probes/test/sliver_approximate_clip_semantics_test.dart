import 'package:flutter/rendering.dart';
import 'package:flutter/semantics.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Clip extends CustomClipper<Rect> {
  _Clip(this.semanticsTop) : super(reclip: semanticsTop);

  final ValueNotifier<double> semanticsTop;

  @override
  Rect getClip(Size size) => Rect.fromLTRB(0, 20, size.width, size.height);

  @override
  Rect getApproximateClipRect(Size size) =>
      Rect.fromLTRB(0, semanticsTop.value, size.width, size.height);

  @override
  bool shouldReclip(_Clip oldClipper) => false;
}

Widget _label(String label, double height) => Semantics(
  label: label,
  container: true,
  child: SizedBox(height: height, width: 800),
);

void main() {
  testWidgets(
    'sliver semantics use the clipper approximation rather than paint shape',
    (tester) async {
      final semantics = tester.ensureSemantics();
      final top = ValueNotifier<double>(50);
      addTearDown(top.dispose);
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: CustomScrollView(
            slivers: [
              SliverClipRect(
                clipper: _Clip(top),
                sliver: SliverToBoxAdapter(
                  child: Column(
                    children: [
                      _label('before', 40),
                      _label('crossing', 30),
                      _label('after', 30),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      );
      final owner = tester.binding.renderViews.single.owner!.semanticsOwner!;
      Map<String, SemanticsNode> labels() {
        final found = <String, SemanticsNode>{};
        void walk(SemanticsNode node) {
          final label = node.getSemanticsData().label;
          if (label.isNotEmpty) {
            found[label] = node;
          }
          node.visitChildren((child) {
            walk(child);
            return true;
          });
        }

        walk(owner.rootSemanticsNode!);
        return found;
      }

      final before = labels();
      expect(before.keys, unorderedEquals(['crossing', 'after']));
      expect(before['crossing']!.rect, const Rect.fromLTRB(0, 10, 800, 30));
      expect(before['after']!.rect, const Rect.fromLTRB(0, 0, 800, 30));

      top.value = 0;
      await tester.pump();
      final after = labels();
      expect(after.keys, unorderedEquals(['before', 'crossing', 'after']));
      expect(after['crossing']!.rect, const Rect.fromLTRB(0, 0, 800, 30));
      expect(after['crossing']!.id, before['crossing']!.id);
      final render = tester.renderObject<RenderSliverClipRect>(
        find.byType(SliverClipRect),
      );
      expect(
        render.paint,
        paints..clipRect(rect: const Rect.fromLTRB(0, 20, 800, 100)),
      );
      semantics.dispose();
    },
  );
}
