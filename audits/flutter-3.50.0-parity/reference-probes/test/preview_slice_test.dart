import 'package:flutter/widget_previews.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

final class MarkTheme extends PreviewThemeData {
  const MarkTheme(this.name);
  final String name;
  @override
  Widget apply(BuildContext context, Widget child) =>
      KeyedSubtree(key: ValueKey(name), child: child);
}

void main() {
  test('preview wrapper newest outermost and initial group', () {
    final builder = PreviewBuilder();
    builder.addWrapper(
      (child) => KeyedSubtree(key: const ValueKey('old'), child: child),
    );
    builder.addWrapper(
      (child) => KeyedSubtree(key: const ValueKey('new'), child: child),
    );
    final preview = builder.build();
    expect(preview.group, 'Default');
    const leaf = SizedBox();
    final outer = preview.wrapper!(leaf) as KeyedSubtree;
    expect(outer.key, const ValueKey('new'));
    expect(outer.child.key, const ValueKey('old'));
    expect((outer.child as KeyedSubtree).child, same(leaf));
  });
  testWidgets('first preview theme is outermost', (t) async {
    const leaf = SizedBox();
    Widget? result;
    await t.pumpWidget(
      Builder(
        builder: (context) {
          result = const MultiPreviewThemeData([
            MarkTheme('first'),
            MarkTheme('second'),
          ]).apply(context, leaf);
          return result!;
        },
      ),
    );
    final outer = result! as KeyedSubtree;
    expect(outer.key, const ValueKey('first'));
    expect(outer.child.key, const ValueKey('second'));
    expect((outer.child as KeyedSubtree).child, same(leaf));
  });
}
