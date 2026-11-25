import 'package:flutter/semantics.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('tagForChildren accumulates until an independent node', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    const oldTag = SemanticsTag('old');
    const newTag = SemanticsTag('new');
    const intermediate = SemanticsTag('pass-through');
    const parent = ValueKey<String>('parent');
    const child = ValueKey<String>('child');
    const leaf = ValueKey<String>('leaf');

    Widget build(SemanticsTag? tag) => Directionality(
      textDirection: TextDirection.ltr,
      child: Semantics(
        key: parent,
        container: true,
        explicitChildNodes: true,
        label: 'parent',
        tagForChildren: tag,
        child: Semantics(
          tagForChildren: intermediate,
          child: Semantics(
            key: child,
            label: 'child',
            container: true,
            explicitChildNodes: true,
            child: Semantics(
              key: leaf,
              label: 'leaf',
              container: true,
              child: const SizedBox.square(dimension: 20),
            ),
          ),
        ),
      ),
    );
    SemanticsNode node(Key key) => tester.getSemantics(find.byKey(key));

    await tester.pumpWidget(build(oldTag));
    expect(node(parent).tags, isNot(contains(oldTag)));
    expect(node(child).tags, containsAll(<SemanticsTag>[oldTag, intermediate]));
    expect(node(leaf).tags, isNot(contains(oldTag)));
    final id = node(child).id;

    await tester.pumpWidget(build(newTag));
    expect(node(child).id, id);
    expect(node(child).tags, containsAll(<SemanticsTag>[newTag, intermediate]));
    expect(node(child).tags, isNot(contains(oldTag)));
    handle.dispose();
  });
}
