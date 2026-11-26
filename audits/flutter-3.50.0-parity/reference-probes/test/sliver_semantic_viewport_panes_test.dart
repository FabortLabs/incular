import 'dart:typed_data';

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Pinned header changes panes only after losing layout extent', (
    WidgetTester tester,
  ) async {
    const ValueKey<String> headerKey = ValueKey<String>('pinned header');
    const ValueKey<String> bodyKey = ValueKey<String>('scrolling content');
    final ScrollController controller = ScrollController();
    addTearDown(controller.dispose);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: CustomScrollView(
          controller: controller,
          slivers: <Widget>[
            const SliverToBoxAdapter(child: SizedBox(height: 100)),
            PinnedHeaderSliver(
              child: Semantics(
                key: headerKey,
                container: true,
                label: 'pinned header',
                child: const SizedBox(height: 40),
              ),
            ),
            SliverToBoxAdapter(
              child: Semantics(
                key: bodyKey,
                container: true,
                label: 'scrolling content',
                child: const SizedBox(height: 1200),
              ),
            ),
          ],
        ),
      ),
    );

    final SemanticsNode initialHeader = tester.getSemantics(
      find.byKey(headerKey),
    );
    final SemanticsNode initialBody = tester.getSemantics(find.byKey(bodyKey));
    final SemanticsNode inner = initialBody.parent!;
    final SemanticsNode outer = inner.parent!;
    final int innerId = inner.id;
    final int outerId = outer.id;
    final int headerId = initialHeader.id;
    for (final double offset in <double>[0, 99, 100, 101, 130, 100, 0]) {
      controller.jumpTo(offset);
      await tester.pump();
      final SemanticsNode header = tester.getSemantics(find.byKey(headerKey));
      final SemanticsNode body = tester.getSemantics(find.byKey(bodyKey));
      final SemanticsNode currentInner = body.parent!;
      final SemanticsNode currentOuter = currentInner.parent!;
      final bool excluded = offset > 100;
      expect(header.id, headerId);
      expect(currentInner.id, innerId);
      expect(currentOuter.id, outerId);
      expect(currentInner.rect, currentOuter.rect);
      expect(currentInner.getSemanticsData().scrollPosition, offset);
      expect(
        currentInner.getSemanticsData().flagsCollection.hasImplicitScrolling,
        isTrue,
      );
      expect(
        currentInner.getSemanticsData().hasAction(
          SemanticsAction.scrollToOffset,
        ),
        isTrue,
      );
      expect(
        currentOuter.getSemanticsData().flagsCollection.hasImplicitScrolling,
        isFalse,
      );
      expect(
        currentOuter.getSemanticsData().hasAction(
          SemanticsAction.scrollToOffset,
        ),
        isFalse,
      );
      final SemanticsNode taggedHeader =
          header.isTagged(RenderViewport.useTwoPaneSemantics)
          ? header
          : header.parent!;
      expect(
        taggedHeader.parent?.id,
        excluded ? outerId : innerId,
        reason: 'offset $offset',
      );
      expect(taggedHeader.isTagged(RenderViewport.useTwoPaneSemantics), isTrue);
      expect(
        taggedHeader.isTagged(RenderViewport.excludeFromScrolling),
        excluded,
      );
      expect(body.isTagged(RenderViewport.excludeFromScrolling), isFalse);
      expect(
        tester.getTopLeft(find.byKey(headerKey)).dy,
        offset < 100 ? 100 - offset : 0,
      );
    }
  });

  testWidgets(
    'Empty viewport keeps its outer node while retiring the scrolling pane',
    (WidgetTester tester) async {
      const ValueKey<String> bodyKey = ValueKey<String>('body');
      final ScrollController controller = ScrollController();
      addTearDown(controller.dispose);
      Widget scene(bool content) {
        return Directionality(
          textDirection: TextDirection.ltr,
          child: CustomScrollView(
            controller: controller,
            slivers: <Widget>[
              if (content)
                SliverToBoxAdapter(
                  child: Semantics(
                    key: bodyKey,
                    label: 'body',
                    child: const SizedBox(height: 1600),
                  ),
                ),
            ],
          ),
        );
      }

      await tester.pumpWidget(scene(false));
      final SemanticsOwner owner = tester.binding.pipelineOwner.semanticsOwner!;
      SemanticsNode? scroller(SemanticsNode node) {
        if (node.getSemanticsData().flagsCollection.hasImplicitScrolling) {
          return node;
        }
        SemanticsNode? found;
        node.visitChildren((SemanticsNode child) {
          found = scroller(child);
          return found == null;
        });
        return found;
      }

      final int outerId = scroller(owner.rootSemanticsNode!)!.id;

      Future<int> attach() async {
        await tester.pumpWidget(scene(true));
        await tester.pumpAndSettle();
        final SemanticsNode pane = scroller(owner.rootSemanticsNode!)!;
        final SemanticsNode body = tester.getSemantics(find.byKey(bodyKey));
        expect(body.parent?.id, pane.id);
        final SemanticsNode boundary = pane.parent!;
        expect(boundary.id, outerId);
        expect(pane.rect, boundary.rect);
        expect(
          boundary.getSemanticsData().hasAction(SemanticsAction.scrollToOffset),
          isFalse,
        );
        expect(
          pane.getSemanticsData().hasAction(SemanticsAction.scrollToOffset),
          isTrue,
        );
        return pane.id;
      }

      final int pane = await attach();
      owner.performAction(
        outerId,
        SemanticsAction.scrollToOffset,
        Float64List.fromList(<double>[0, 220]),
      );
      expect(controller.offset, 0);
      owner.performAction(
        pane,
        SemanticsAction.scrollToOffset,
        Float64List.fromList(<double>[0, 220]),
      );
      await tester.pump();
      expect(controller.offset, 220);

      await tester.pumpWidget(scene(false));
      await tester.pumpAndSettle();
      expect(scroller(owner.rootSemanticsNode!)!.id, outerId);
      owner.performAction(
        pane,
        SemanticsAction.scrollToOffset,
        Float64List.fromList(<double>[0, 280]),
      );
      expect(controller.offset, 0);
      final int replacement = await attach();
      expect(replacement, isNot(pane));
    },
  );
}
