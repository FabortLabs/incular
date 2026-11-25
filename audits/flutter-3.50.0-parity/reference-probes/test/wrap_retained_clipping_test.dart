import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Wrap clips overflow only in its current clip mode', (
    tester,
  ) async {
    Widget wrap(int count, Clip mode) => Wrap(
      textDirection: TextDirection.ltr,
      clipBehavior: mode,
      children: List.filled(count, const SizedBox(width: 500, height: 500)),
    );
    await tester.pumpWidget(wrap(2, Clip.none));
    RenderWrap render() => tester.renderObject(find.byType(Wrap));
    final original = render();
    expect(render(), isNot(paints..clipRect()));
    await tester.pumpWidget(wrap(2, Clip.hardEdge));
    expect(render(), same(original));
    expect(render(), paints..clipRect());
    await tester.pumpWidget(wrap(2, Clip.antiAlias));
    expect(render(), same(original));
    expect(render().clipBehavior, Clip.antiAlias);
    expect(render(), paints..clipRect());
    await tester.pumpWidget(wrap(2, Clip.antiAliasWithSaveLayer));
    expect(render(), same(original));
    expect(render().clipBehavior, Clip.antiAliasWithSaveLayer);
    expect(render(), paints..clipRect());
    await tester.pumpWidget(wrap(1, Clip.antiAliasWithSaveLayer));
    expect(render(), same(original));
    expect(render(), isNot(paints..clipRect()));
    await tester.pumpWidget(wrap(2, Clip.none));
    expect(render(), same(original));
    expect(render(), isNot(paints..clipRect()));
  });
}
