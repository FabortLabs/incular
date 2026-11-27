import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _First extends Intent {
  const _First();
}

class _Second extends Intent {}

class _Toggle<I extends Intent> extends Action<I> {
  bool _enabled = true;

  set enabled(bool value) {
    if (value != _enabled) {
      _enabled = value;
      notifyActionListeners();
    }
  }

  @override
  bool isEnabled(covariant I intent) => _enabled;

  @override
  void invoke(covariant I intent) {}
}

void main() {
  testWidgets(
    'changes rebuild only readers of the action-owning scope',
    (tester) async {
      final first = _Toggle<_First>();
      final unrelated = _Toggle<_Second>();
      final reads = <bool?>[];
      var unrelatedBuilds = 0;
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Actions(
            actions: <Type, Action<Intent>>{_First: first},
            child: Actions(
              actions: <Type, Action<Intent>>{_Second: unrelated},
              child: Column(
                children: [
                  Builder(
                    builder: (context) {
                      reads.add(Actions.maybeFind<_First>(context)?.isEnabled(const _First()));
                      return const SizedBox.shrink();
                    },
                  ),
                  Builder(
                    builder: (context) {
                      unrelatedBuilds += 1;
                      return const SizedBox.shrink();
                    },
                  ),
                ],
              ),
            ),
          ),
        ),
      );
      expect(reads, [true]);
      expect(unrelatedBuilds, 1);

      unrelated.enabled = false;
      await tester.pump();
      expect(reads, [true]);

      first.enabled = false;
      await tester.pump();
      expect(reads, [true, false]);
      expect(unrelatedBuilds, 1);
    },
  );
}
