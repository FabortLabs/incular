import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class LiveDelegate extends SingleChildLayoutDelegate {
  LiveDelegate(this.size) : super(relayout: size);

  final ValueNotifier<Size> size;

  @override
  Size getSize(BoxConstraints constraints) => size.value;

  @override
  bool shouldRelayout(covariant LiveDelegate oldDelegate) => false;
}

void main() {
  testWidgets('a replacement can keep geometry but switch relayout sources', (
    tester,
  ) async {
    const key = ValueKey('layout');
    final first = ValueNotifier(const Size(100, 100));
    final second = ValueNotifier(const Size(180, 140));
    addTearDown(first.dispose);
    addTearDown(second.dispose);
    Widget scene(ValueNotifier<Size> size) => Center(
      child: CustomSingleChildLayout(
        key: key,
        delegate: LiveDelegate(size),
        child: const SizedBox(),
      ),
    );
    final layout = find.byKey(key);
    await tester.pumpWidget(scene(first));
    expect(tester.getSize(layout), const Size(100, 100));
    await tester.pumpWidget(scene(second));
    expect(tester.getSize(layout), const Size(100, 100));
    first.value = const Size(50, 50);
    await tester.pump();
    expect(tester.getSize(layout), const Size(100, 100));
    second.value = const Size(200, 160);
    await tester.pump();
    expect(tester.getSize(layout), const Size(200, 160));
  });
}
