import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class Notifier extends ChangeNotifier {
  bool get listening => hasListeners;
  void fire() => notifyListeners();
}

void main() {
  testWidgets('replacing and unmounting releases the prior notifier', (
    tester,
  ) async {
    final before = Notifier();
    final after = Notifier();
    addTearDown(before.dispose);
    addTearDown(after.dispose);
    var builds = 0;
    Widget root(Notifier notifier) => ListenableBuilder(
      listenable: notifier,
      builder: (_, _) {
        builds++;
        return const SizedBox();
      },
    );
    await tester.pumpWidget(root(before));
    expect(before.listening, isTrue);
    expect(after.listening, isFalse);
    await tester.pumpWidget(root(after));
    expect(builds, 2);
    expect(before.listening, isFalse);
    expect(after.listening, isTrue);
    before.fire();
    expect(tester.binding.hasScheduledFrame, isFalse);
    after.fire();
    expect(tester.binding.hasScheduledFrame, isTrue);
    await tester.pump();
    expect(builds, 3);
    await tester.pumpWidget(const SizedBox());
    expect(after.listening, isFalse);
  });

  testWidgets('a post-frame notification schedules the next build', (
    tester,
  ) async {
    final notifier = Notifier();
    addTearDown(notifier.dispose);
    var builds = 0;
    await tester.pumpWidget(
      ListenableBuilder(
        listenable: notifier,
        builder: (_, _) {
          builds++;
          return const SizedBox();
        },
      ),
    );
    WidgetsBinding.instance.addPostFrameCallback((_) => notifier.fire());
    tester.binding.scheduleFrame();
    await tester.pump();
    expect(builds, 1);
    expect(tester.binding.hasScheduledFrame, isTrue);
    await tester.pump();
    expect(builds, 2);
    expect(tester.binding.hasScheduledFrame, isFalse);
  });
}
