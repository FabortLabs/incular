import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'Viewport tag crosses roleless wrappers but stops at a semantic node',
    (WidgetTester tester) async {
      const ValueKey<String> innerKey = ValueKey<String>('inside');
      const SemanticsTag custom = SemanticsTag('custom');
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: CustomScrollView(
            slivers: <Widget>[
              SliverToBoxAdapter(
                child: Padding(
                  padding: EdgeInsets.all(10),
                  child: Text('direct'),
                ),
              ),
              SliverSemantics(
                container: true,
                label: 'boundary',
                explicitChildNodes: true,
                tagForChildren: custom,
                sliver: SliverToBoxAdapter(
                  child: Semantics(
                    key: innerKey,
                    container: true,
                    label: 'inside',
                    child: SizedBox(height: 20),
                  ),
                ),
              ),
            ],
          ),
        ),
      );
      final SemanticsNode direct = tester.getSemantics(find.text('direct'));
      final SemanticsNode inner = tester.getSemantics(find.byKey(innerKey));
      final SemanticsNode boundary = inner.parent!;
      final SemanticsNode viewport = direct.parent!;
      void checkTags() {
        expect(direct.isTagged(RenderViewport.useTwoPaneSemantics), isTrue);
        expect(boundary.isTagged(RenderViewport.useTwoPaneSemantics), isTrue);
        expect(boundary.isTagged(custom), isFalse);
        expect(inner.isTagged(RenderViewport.useTwoPaneSemantics), isFalse);
        expect(inner.isTagged(custom), isTrue);
        expect(viewport.isTagged(RenderViewport.useTwoPaneSemantics), isFalse);
        expect(
          viewport.getSemanticsData().flagsCollection.hasImplicitScrolling,
          isTrue,
        );
      }

      checkTags();
      await tester.pump();
      expect(tester.getSemantics(find.byKey(innerKey)).id, inner.id);
      checkTags();
    },
  );
}
